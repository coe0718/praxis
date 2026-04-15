use std::{convert::Infallible, path::PathBuf, time::Duration};

use anyhow::Result;
use async_stream::stream;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use serde_json::json;

use crate::{
    cli::core,
    events::{Event as PraxisEvent, read_events_since},
    mcp::{server::dispatch, types::JsonRpcRequest},
    paths::PraxisPaths,
    report::build_status_report,
    wakeup::{WakeIntent, request_wake},
};
#[cfg(feature = "discord")]
use crate::messaging::discord::DiscordInteraction;
#[cfg(feature = "slack")]
use crate::messaging::slack::SlackEvent;


#[derive(Clone)]
struct DashboardState {
    data_dir: PathBuf,
}

pub async fn serve_dashboard(data_dir: PathBuf, host: String, port: u16) -> Result<()> {
    let state = DashboardState { data_dir };
    let app = Router::new()
        .route("/", get(index))
        .route("/status", get(status))
        .route("/summary", get(summary))
        .route("/health", get(health))
        .route("/metrics", get(prometheus_metrics))
        .route("/events/recent", get(recent_events))
        .route("/events", get(events))
        .route("/mcp", post(mcp_endpoint));

    #[cfg(feature = "discord")]
    let app = app.route("/webhook/discord", post(webhook_discord));
    #[cfg(feature = "slack")]
    let app = app.route("/webhook/slack", post(webhook_slack));

    let app = app.with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Praxis</title></head>
<body style="font-family: sans-serif; margin: 2rem;">
<h1>Praxis Dashboard</h1>
<pre id="status">loading...</pre>
<pre id="events" style="height: 20rem; overflow: auto; border: 1px solid #ccc; padding: 1rem;"></pre>
<script>
async function refreshStatus() {
  const summary = await fetch('/summary').then(r => r.json());
  document.getElementById('status').textContent = JSON.stringify(summary, null, 2);
}
refreshStatus();
setInterval(refreshStatus, 3000);
const events = document.getElementById('events');
fetch('/events/recent').then(r => r.json()).then(items => {
  items.forEach(item => {
    events.textContent += item.kind + ": " + item.detail + "\n";
  });
});
const source = new EventSource('/events');
source.onmessage = (event) => {
  events.textContent += event.data + "\n";
  events.scrollTop = events.scrollHeight;
};
</script></body></html>"#,
    )
}

async fn status(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_status(Some(state.data_dir))
        .unwrap_or_else(|error| format!("status error: {error}"))
}

async fn summary(State(state): State<DashboardState>) -> impl IntoResponse {
    match core::load_initialized_config(Some(state.data_dir.clone()))
        .and_then(|(config, paths)| build_status_report(&config, &paths))
    {
        Ok(report) => Json(report).into_response(),
        Err(error) => format!("summary error: {error}").into_response(),
    }
}

async fn health(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_doctor(Some(state.data_dir))
        .unwrap_or_else(|error| format!("health error: {error}"))
}

async fn recent_events(State(state): State<DashboardState>) -> impl IntoResponse {
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

async fn events(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let path = state.data_dir.join("events.jsonl");
    let stream = stream! {
        let mut offset = 0;
        loop {
            let (events, next_offset) = read_events_since(&path, offset)
                .unwrap_or_else(|_| (Vec::new(), offset));
            offset = next_offset;
            for item in events {
                yield Ok(Event::default().event(item.kind).data(item.detail));
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    };
    Sse::new(stream)
}

#[cfg(feature = "discord")]
/// Discord interactions endpoint.
///
/// Handles PING (type=1) for endpoint verification and APPLICATION_COMMAND
/// (type=2) by injecting a wake intent so the agent loop picks up the request.
async fn webhook_discord(
    State(state): State<DashboardState>,
    Json(body): Json<DiscordInteraction>,
) -> impl IntoResponse {
    // Respond to Discord's PING challenge immediately.
    if body.interaction_type == 1 {
        return (StatusCode::OK, Json(json!({ "type": 1 }))).into_response();
    }

    // Extract a text command from the interaction data if present.
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
        log::warn!("discord webhook: failed to write wake intent: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "type": 4, "data": { "content": "internal error" } })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({ "type": 5 })), // DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
    )
        .into_response()
}

#[cfg(feature = "slack")]
/// Slack Events API endpoint.
///
/// Handles `url_verification` challenges and `message` events by injecting
/// a wake intent for the agent loop.
async fn webhook_slack(
    State(state): State<DashboardState>,
    Json(body): Json<SlackEvent>,
) -> impl IntoResponse {
    // Respond to Slack's URL verification challenge.
    if body.event_type == "url_verification" {
        let challenge = body.challenge.as_deref().unwrap_or("");
        return (StatusCode::OK, Json(json!({ "challenge": challenge }))).into_response();
    }

    if body.event_type == "event_callback" {
        if let Some(event) = &body.event {
            let text = event.text.as_deref().unwrap_or("slack event").to_string();
            let mut intent = WakeIntent::new(&text, "slack");
            intent = intent.with_task(text.clone());

            if let Err(e) = request_wake(&state.data_dir, &intent) {
                log::warn!("slack webhook: failed to write wake intent: {e}");
            }
        }
    }

    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

/// Prometheus text-format metrics endpoint (`/metrics`).
///
/// Exposes a small set of operational gauges in the standard Prometheus
/// exposition format.  No external SDK is required — the format is plain text.
async fn prometheus_metrics(State(state): State<DashboardState>) -> impl IntoResponse {
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

    // Hot memory count.
    let hot_count = store
        .recent_hot_memories(10_000)
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_hot_memory_count",
        "Number of hot memories in the store.",
        hot_count
    );

    // Cold memory count.
    let cold_count = store
        .strongest_cold_memories(10_000)
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_cold_memory_count",
        "Number of cold memories in the store.",
        cold_count
    );

    // Pending approvals.
    let pending = store
        .list_approvals(Some(ApprovalStatus::Pending))
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!(
        "praxis_approvals_pending",
        "Number of tool-approval requests awaiting operator action.",
        pending
    );

    // Heartbeat age — parse the timestamp string from the record.
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
        "Seconds since the last runtime heartbeat was written (-1 = never).",
        heartbeat_age_secs
    );

    // Session count from last session ID (proxy for total sessions).
    let session_count = store
        .last_session()
        .ok()
        .flatten()
        .map(|s| s.id)
        .unwrap_or(0);
    gauge!(
        "praxis_sessions_total",
        "Total sessions recorded (approximated by last session ID).",
        session_count
    );

    lines.push(String::new()); // trailing newline
    lines.join("\n")
}

/// MCP JSON-RPC endpoint — exposes Praxis tools and resources to any MCP client.
async fn mcp_endpoint(
    State(state): State<DashboardState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let response = dispatch(&paths, &request);
    (StatusCode::OK, Json(response)).into_response()
}
