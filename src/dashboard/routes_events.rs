use std::{convert::Infallible, time::Duration};

use async_stream::stream;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response, Sse, sse::Event},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    events::{Event as PraxisEvent, read_events_since},
    mcp::{server::dispatch, types::JsonRpcRequest},
    paths::PraxisPaths,
};

use super::server::DashboardState;

pub(super) async fn index() -> Html<&'static str> {
    Html(
        r#"<!doctype html><html><body><p>Praxis — React UI served separately on port 5173</p></body></html>"#,
    )
}

pub(super) async fn status_text(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::cli::core;
    core::handle_status(Some(state.data_dir)).unwrap_or_else(|e| format!("status error: {e}"))
}

pub(super) async fn health(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::cli::core;
    core::handle_doctor(Some(state.data_dir)).unwrap_or_else(|e| format!("health error: {e}"))
}

pub(super) async fn recent_events(State(state): State<DashboardState>) -> impl IntoResponse {
    let path = state.data_dir.join("events.jsonl");
    let events = read_events_since(&path, 0)
        .map(|(items, _)| {
            items
                .into_iter()
                .rev()
                .take(20)
                .collect::<Vec<PraxisEvent>>()
        })
        .unwrap_or_default();
    Json(events.into_iter().rev().collect::<Vec<_>>())
}

#[derive(Deserialize)]
pub(super) struct SseToken {
    token: Option<String>,
}

/// SECURITY NOTE: The SSE endpoint accepts the auth token as a query
/// parameter because the browser `EventSource` API cannot send custom
/// headers. This means the token may appear in server access logs.
/// Operators should ensure log access is restricted.
pub(super) async fn events_sse(
    State(state): State<DashboardState>,
    Query(params): Query<SseToken>,
) -> Response {
    if let Some(ref expected) = state.token {
        let provided = params.token.as_deref().unwrap_or("");
        if provided != expected {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
    let path = state.data_dir.join("events.jsonl");
    let s = stream! {
        let mut offset = 0;
        loop {
            let (events, next) = read_events_since(&path, offset).unwrap_or((Vec::new(), offset));
            offset = next;
            for item in events {
                yield Ok::<Event, Infallible>(Event::default().event(item.kind).data(item.detail));
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    };
    Sse::new(s).into_response()
}

pub(super) async fn mcp_endpoint(
    State(state): State<DashboardState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let response = dispatch(&paths, &request);
    (StatusCode::OK, Json(response)).into_response()
}

pub(super) async fn prometheus_metrics(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let body = collect_prometheus_metrics(&paths);
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
}

fn collect_prometheus_metrics(paths: &PraxisPaths) -> String {
    use crate::{
        heartbeat::read_heartbeat,
        memory::MemoryStore,
        storage::{ApprovalStatus, ApprovalStore, SessionStore, SqliteSessionStore},
    };
    use chrono::{DateTime, Utc};

    let store = SqliteSessionStore::new(paths.database_file.clone());
    let now = Utc::now();
    let mut lines = Vec::new();

    macro_rules! gauge {
        ($name:expr, $help:expr, $val:expr) => {
            lines.push(format!("# HELP {} {}", $name, $help));
            lines.push(format!("# TYPE {} gauge", $name));
            lines.push(format!("{} {}", $name, $val));
        };
    }

    let hot_count = store
        .recent_hot_memories(10_000)
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_hot_memory_count",
        "Number of hot memories.",
        hot_count
    );

    let cold_count = store
        .strongest_cold_memories(10_000)
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_cold_memory_count",
        "Number of cold memories.",
        cold_count
    );

    let pending = store
        .list_approvals(Some(ApprovalStatus::Pending))
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_approvals_pending",
        "Pending tool-approval requests.",
        pending
    );

    let heartbeat_age_secs: i64 = read_heartbeat(&paths.heartbeat_file)
        .ok()
        .and_then(|hb| {
            DateTime::parse_from_rfc3339(&hb.updated_at)
                .ok()
                .map(|ts| (now - ts.with_timezone(&Utc)).num_seconds().max(0))
        })
        .unwrap_or(-1);
    gauge!(
        "praxis_heartbeat_age_seconds",
        "Seconds since last heartbeat (-1=never).",
        heartbeat_age_secs
    );

    let session_count = store
        .last_session()
        .ok()
        .flatten()
        .map(|s| s.id)
        .unwrap_or(0);
    gauge!(
        "praxis_sessions_total",
        "Total sessions (by last session ID).",
        session_count
    );

    lines.push(String::new());
    lines.join("\n")
}

#[cfg(feature = "discord")]
pub(super) async fn webhook_discord(
    State(state): State<DashboardState>,
    Json(body): Json<crate::messaging::discord::DiscordInteraction>,
) -> impl IntoResponse {
    use crate::wakeup::{WakeIntent, request_wake};
    if body.interaction_type == 1 {
        return (StatusCode::OK, Json(json!({ "type": 1 }))).into_response();
    }
    let task = body
        .data
        .as_ref()
        .and_then(|d| d.name.as_deref())
        .map(str::to_string);
    let reason = task
        .clone()
        .unwrap_or_else(|| "discord interaction".to_string());
    let mut intent = WakeIntent::new(&reason, "discord");
    if let Some(t) = task {
        intent = intent.with_task(t);
    }
    if let Err(e) = request_wake(&state.data_dir, &intent) {
        log::warn!("discord webhook: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "type": 4, "data": { "content": "internal error" } })),
        )
            .into_response();
    }
    (StatusCode::OK, Json(json!({ "type": 5 }))).into_response()
}

#[cfg(feature = "slack")]
pub(super) async fn webhook_slack(
    State(state): State<DashboardState>,
    Json(body): Json<crate::messaging::slack::SlackEvent>,
) -> impl IntoResponse {
    use crate::wakeup::{WakeIntent, request_wake};
    if body.event_type == "url_verification" {
        let challenge = body.challenge.as_deref().unwrap_or("");
        return (StatusCode::OK, Json(json!({ "challenge": challenge }))).into_response();
    }
    if body.event_type == "event_callback"
        && let Some(event) = &body.event
    {
        let text = event.text.as_deref().unwrap_or("slack event").to_string();
        let intent = WakeIntent::new(&text, "slack").with_task(text.clone());
        if let Err(e) = request_wake(&state.data_dir, &intent) {
            log::warn!("slack webhook: {e}");
        }
    }
    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}
