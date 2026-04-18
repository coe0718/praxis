use std::{convert::Infallible, path::PathBuf, time::Duration};

use anyhow::Result;
use async_stream::stream;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Sse, sse::Event},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[cfg(feature = "discord")]
use crate::messaging::discord::DiscordInteraction;
#[cfg(feature = "slack")]
use crate::messaging::slack::SlackEvent;
use crate::{
    cli::core,
    events::{Event as PraxisEvent, read_events_since},
    mcp::{server::dispatch, types::JsonRpcRequest},
    paths::PraxisPaths,
    report::build_status_report,
    wakeup::{WakeIntent, request_wake},
};

// ── API response types ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApprovalRow {
    id: i64,
    tool_name: String,
    summary: String,
    requested_by: String,
    write_paths: Vec<String>,
    payload_json: Option<String>,
    status: String,
    status_note: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct MemoryRow {
    id: i64,
    tier: String,
    content: String,
    summary: Option<String>,
    tags: Vec<String>,
    score: f64,
    memory_type: String,
}

#[derive(Serialize)]
struct GoalRow {
    raw_id: String,
    title: String,
    completed: bool,
}

#[derive(Deserialize)]
struct AddGoalBody {
    description: String,
}

#[derive(Deserialize)]
struct WakeBody {
    task: Option<String>,
    reason: Option<String>,
    urgent: Option<bool>,
}

#[derive(Deserialize)]
struct RunBody {
    task: Option<String>,
}

#[derive(Deserialize)]
struct WriteFileBody {
    content: String,
}

// ── State ──────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct DashboardState {
    data_dir: PathBuf,
}

// ── Router ─────────────────────────────────────────────────────────────────────

pub async fn serve_dashboard(data_dir: PathBuf, host: String, port: u16) -> Result<()> {
    let state = DashboardState { data_dir };
    let app = Router::new()
        // Dashboard SPA
        .route("/", get(index))
        // Legacy compatibility
        .route("/status", get(status_text))
        .route("/health", get(health))
        .route("/metrics", get(prometheus_metrics))
        .route("/events/recent", get(recent_events))
        .route("/events", get(events_sse))
        .route("/mcp", post(mcp_endpoint))
        // REST API
        .route("/api/summary", get(api_summary))
        .route("/api/sessions", get(api_sessions))
        .route("/api/approvals", get(api_approvals))
        .route("/api/approvals/:id/approve", post(api_approve))
        .route("/api/approvals/:id/reject", post(api_reject))
        .route("/api/memories/hot", get(api_memories_hot))
        .route("/api/memories/cold", get(api_memories_cold))
        .route("/api/memories/consolidate", post(api_memories_consolidate))
        .route("/api/memories/:id/reinforce", post(api_memory_reinforce))
        .route("/api/memories/:id/forget", post(api_memory_forget))
        .route("/api/tools", get(api_tools))
        .route("/api/goals", get(api_goals))
        .route("/api/goals", post(api_goals_add))
        .route("/api/identity/:file", get(api_identity_read))
        .route("/api/identity/:file", put(api_identity_write))
        .route("/api/config", get(api_config))
        .route("/api/canary", get(api_canary))
        .route("/api/heartbeat", get(api_heartbeat))
        .route("/api/score", get(api_score))
        .route("/api/evolution", get(api_evolution))
        .route("/api/evolution/:id/approve", post(api_evolution_approve))
        .route("/api/delegation", get(api_delegation))
        .route("/api/wake", post(api_wake))
        .route("/api/run", post(api_run));

    #[cfg(feature = "discord")]
    let app = app.route("/webhook/discord", post(webhook_discord));
    #[cfg(feature = "slack")]
    let app = app.route("/webhook/slack", post(webhook_slack));

    let app = app.with_state(state);
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    log::info!("dashboard: listening on http://{host}:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

// ── SPA ────────────────────────────────────────────────────────────────────────

async fn index() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// ── Legacy endpoints ───────────────────────────────────────────────────────────

async fn status_text(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_status(Some(state.data_dir)).unwrap_or_else(|e| format!("status error: {e}"))
}

async fn health(State(state): State<DashboardState>) -> impl IntoResponse {
    core::handle_doctor(Some(state.data_dir)).unwrap_or_else(|e| format!("health error: {e}"))
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

async fn events_sse(
    State(state): State<DashboardState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let path = state.data_dir.join("events.jsonl");
    let s = stream! {
        let mut offset = 0;
        loop {
            let (events, next) = read_events_since(&path, offset).unwrap_or((Vec::new(), offset));
            offset = next;
            for item in events {
                yield Ok(Event::default().event(item.kind).data(item.detail));
            }
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
    };
    Sse::new(s)
}

async fn mcp_endpoint(
    State(state): State<DashboardState>,
    Json(request): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let response = dispatch(&paths, &request);
    (StatusCode::OK, Json(response)).into_response()
}

// ── API handlers ───────────────────────────────────────────────────────────────

async fn api_summary(State(state): State<DashboardState>) -> impl IntoResponse {
    match core::load_initialized_config(Some(state.data_dir.clone()))
        .and_then(|(config, paths)| build_status_report(&config, &paths))
    {
        Ok(report) => Json(serde_json::to_value(report).unwrap_or(json!({}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_sessions(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match query_recent_sessions(&paths.database_file, 50) {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_approvals(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::storage::{ApprovalStore, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.list_approvals(None) {
        Ok(rows) => {
            let out: Vec<ApprovalRow> = rows
                .into_iter()
                .map(|r| ApprovalRow {
                    id: r.id,
                    tool_name: r.tool_name,
                    summary: r.summary,
                    requested_by: r.requested_by,
                    write_paths: r.write_paths,
                    payload_json: r.payload_json,
                    status: r.status.as_str().to_string(),
                    status_note: r.status_note,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                })
                .collect();
            Json(out).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_approve(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.set_approval_status(id, ApprovalStatus::Approved, None) {
        Ok(Some(r)) => Json(json!({ "id": r.id, "status": r.status.as_str() })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "approval not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_reject(State(state): State<DashboardState>, Path(id): Path<i64>) -> impl IntoResponse {
    use crate::storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.set_approval_status(id, ApprovalStatus::Rejected, None) {
        Ok(Some(r)) => Json(json!({ "id": r.id, "status": r.status.as_str() })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "approval not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_memories_hot(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::memory::MemoryStore;
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.recent_hot_memories(200) {
        Ok(mems) => {
            let rows: Vec<MemoryRow> = mems
                .into_iter()
                .map(|m| MemoryRow {
                    id: m.id,
                    tier: "hot".to_string(),
                    content: m.content,
                    summary: m.summary,
                    tags: m.tags,
                    score: m.score as f64,
                    memory_type: m.memory_type.as_str().to_string(),
                })
                .collect();
            Json(rows).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_memories_cold(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::memory::MemoryStore;
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.strongest_cold_memories(200) {
        Ok(mems) => {
            let rows: Vec<MemoryRow> = mems
                .into_iter()
                .map(|m| MemoryRow {
                    id: m.id,
                    tier: "cold".to_string(),
                    content: m.content,
                    summary: m.summary,
                    tags: m.tags,
                    score: m.score as f64,
                    memory_type: m.memory_type.as_str().to_string(),
                })
                .collect();
            Json(rows).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_memories_consolidate(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::memory::MemoryStore;
    use crate::storage::SqliteSessionStore;
    use chrono::Utc;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.consolidate_memories(Utc::now()) {
        Ok(s) => {
            Json(json!({ "consolidated": s.consolidated, "pruned": s.pruned })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_memory_reinforce(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::memory::MemoryStore;
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.boost_memory(id) {
        Ok(_) => Json(json!({ "id": id, "action": "reinforced" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_memory_forget(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::memory::MemoryStore;
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.forget_memory(id) {
        Ok(_) => Json(json!({ "id": id, "action": "forgotten" })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_tools(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::tools::{FileToolRegistry, ToolRegistry};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match FileToolRegistry.list(&paths) {
        Ok(tools) => Json(tools).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_goals(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match parse_goals_file(&paths.goals_file) {
        Ok(goals) => Json(goals).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_goals_add(
    State(state): State<DashboardState>,
    Json(body): Json<AddGoalBody>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    if body.description.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "description is required").into_response();
    }
    match append_goal(&paths.goals_file, &body.description) {
        Ok(id) => Json(json!({ "id": id, "title": body.description })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_identity_read(
    State(state): State<DashboardState>,
    Path(file): Path<String>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let path = match resolve_identity_file(&paths, &file) {
        Some(p) => p,
        None => {
            return (StatusCode::NOT_FOUND, "unknown identity file").into_response();
        }
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            Json(json!({ "file": file, "content": content, "writable": file != "soul" }))
                .into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Json(json!({ "file": file, "content": "", "writable": file != "soul" })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_identity_write(
    State(state): State<DashboardState>,
    Path(file): Path<String>,
    Json(body): Json<WriteFileBody>,
) -> impl IntoResponse {
    if file == "soul" {
        return (StatusCode::FORBIDDEN, "SOUL.md is immutable").into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let path = match resolve_identity_file(&paths, &file) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "unknown identity file").into_response(),
    };
    match std::fs::write(&path, &body.content) {
        Ok(()) => Json(json!({ "file": file, "saved": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_config(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let mut out = json!({});
    if let Ok(raw) = std::fs::read_to_string(&paths.config_file) {
        out["praxis_toml"] = json!(raw);
    }
    if let Ok(raw) = std::fs::read_to_string(&paths.providers_file) {
        out["providers_toml"] = json!(raw);
    }
    if let Ok(raw) = std::fs::read_to_string(&paths.budgets_file) {
        out["budgets_toml"] = json!(raw);
    }
    Json(out)
}

async fn api_canary(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::canary::{CanaryFreezeState, ModelCanaryLedger};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let ledger = ModelCanaryLedger::load_or_default(&paths.model_canary_file)
        .unwrap_or_else(|_| ModelCanaryLedger { records: vec![] });
    let freeze = CanaryFreezeState::load_or_default(&paths.canary_freeze_file).unwrap_or_default();
    Json(json!({ "records": ledger.records, "frozen": freeze.frozen }))
}

async fn api_heartbeat(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::heartbeat::read_heartbeat;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match read_heartbeat(&paths.heartbeat_file) {
        Ok(hb) => Json(json!({
            "component": hb.component,
            "phase": hb.phase,
            "detail": hb.detail,
            "updated_at": hb.updated_at,
            "pid": hb.pid,
            "process_uptime_ms": hb.process_uptime_ms,
        }))
        .into_response(),
        Err(_) => Json(json!(null)).into_response(),
    }
}

async fn api_score(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match read_jsonl_tail(&paths.score_file, 30) {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_evolution(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = crate::evolution::EvolutionStore::from_paths(&paths);
    match store.all() {
        Ok(proposals) => Json(proposals).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_evolution_approve(
    State(state): State<DashboardState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = crate::evolution::EvolutionStore::from_paths(&paths);
    match store.approve(&id) {
        Ok(proposal) => {
            Json(json!({ "id": id, "approved": true, "status": proposal.status.label() }))
                .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_delegation(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::delegation::DelegationStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match DelegationStore::load(&paths.delegation_links_file) {
        Ok(store) => Json(json!({
            "links": store.links.values().collect::<Vec<_>>(),
            "active_counts": store.active_counts,
        }))
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_wake(
    State(state): State<DashboardState>,
    Json(body): Json<WakeBody>,
) -> impl IntoResponse {
    let reason = body.reason.unwrap_or_else(|| "dashboard wake".to_string());
    let mut intent = WakeIntent::new(&reason, "dashboard");
    if let Some(task) = body.task {
        intent = intent.with_task(task);
    }
    if body.urgent.unwrap_or(false) {
        intent = intent.urgent();
    }
    match request_wake(&state.data_dir, &intent) {
        Ok(()) => Json(json!({ "queued": true, "reason": reason })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn api_run(
    State(state): State<DashboardState>,
    Json(body): Json<RunBody>,
) -> impl IntoResponse {
    use crate::cli::RunArgs;
    use crate::cli::core::handle_run;
    match handle_run(
        Some(state.data_dir.clone()),
        RunArgs {
            once: true,
            force: true,
            task: body.task,
            profile: None,
        },
    ) {
        Ok(summary) => Json(json!({ "outcome": summary })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Prometheus ─────────────────────────────────────────────────────────────────

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

// ── Webhook handlers ───────────────────────────────────────────────────────────

#[cfg(feature = "discord")]
async fn webhook_discord(
    State(state): State<DashboardState>,
    Json(body): Json<DiscordInteraction>,
) -> impl IntoResponse {
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
async fn webhook_slack(
    State(state): State<DashboardState>,
    Json(body): Json<SlackEvent>,
) -> impl IntoResponse {
    if body.event_type == "url_verification" {
        let challenge = body.challenge.as_deref().unwrap_or("");
        return (StatusCode::OK, Json(json!({ "challenge": challenge }))).into_response();
    }
    if body.event_type == "event_callback" {
        if let Some(event) = &body.event {
            let text = event.text.as_deref().unwrap_or("slack event").to_string();
            let intent = WakeIntent::new(&text, "slack").with_task(text.clone());
            if let Err(e) = request_wake(&state.data_dir, &intent) {
                log::warn!("slack webhook: {e}");
            }
        }
    }
    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn query_recent_sessions(db_path: &std::path::Path, limit: usize) -> anyhow::Result<Vec<Value>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, day, session_num, started_at, ended_at, outcome,
                selected_goal_id, selected_goal_title, selected_task, action_summary
         FROM sessions ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "day": row.get::<_, i64>(1)?,
            "session_num": row.get::<_, i64>(2)?,
            "started_at": row.get::<_, String>(3)?,
            "ended_at": row.get::<_, String>(4)?,
            "outcome": row.get::<_, String>(5)?,
            "selected_goal_id": row.get::<_, Option<String>>(6)?,
            "selected_goal_title": row.get::<_, Option<String>>(7)?,
            "selected_task": row.get::<_, Option<String>>(8)?,
            "action_summary": row.get::<_, String>(9)?,
        }))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(anyhow::Error::from)
}

fn parse_goals_file(path: &std::path::Path) -> anyhow::Result<Vec<GoalRow>> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut goals = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            let (raw_id, title) = split_goal_id(rest);
            goals.push(GoalRow {
                raw_id,
                title,
                completed: true,
            });
        } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            let (raw_id, title) = split_goal_id(rest);
            goals.push(GoalRow {
                raw_id,
                title,
                completed: false,
            });
        }
    }
    Ok(goals)
}

fn split_goal_id(s: &str) -> (String, String) {
    if let Some(colon) = s.find(": ") {
        let id = s[..colon].trim().to_string();
        let title = s[colon + 2..].trim().to_string();
        (id, title)
    } else {
        (String::new(), s.trim().to_string())
    }
}

fn append_goal(path: &std::path::Path, description: &str) -> anyhow::Result<String> {
    let contents = std::fs::read_to_string(path).unwrap_or_default();
    let next_id = contents
        .lines()
        .filter_map(|l| l.split("G-").nth(1))
        .filter_map(|r| r.split(':').next())
        .filter_map(|d| d.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let goal_id = format!("G-{next_id:03}");
    let mut updated = contents.trim_end().to_string();
    updated.push_str(&format!("\n- [ ] {goal_id}: {description}\n"));
    std::fs::write(path, updated)?;
    Ok(goal_id)
}

fn resolve_identity_file(paths: &PraxisPaths, name: &str) -> Option<std::path::PathBuf> {
    match name {
        "soul" => Some(paths.soul_file.clone()),
        "identity" => Some(paths.identity_file.clone()),
        "agents" => Some(paths.agents_file.clone()),
        "goals" => Some(paths.goals_file.clone()),
        "journal" => Some(paths.journal_file.clone()),
        "patterns" => Some(paths.patterns_file.clone()),
        "learnings" => Some(paths.learnings_file.clone()),
        "roadmap" => Some(paths.roadmap_file.clone()),
        _ => None,
    }
}

fn read_jsonl_tail(path: &std::path::Path, limit: usize) -> anyhow::Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    let rows: Vec<Value> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let start = rows.len().saturating_sub(limit);
    Ok(rows[start..].to_vec())
}

// ── Dashboard HTML ─────────────────────────────────────────────────────────────

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Praxis</title>
<style>
:root {
  --bg: #0b0b12;
  --bg2: #0f0f18;
  --card: #12121c;
  --card2: #16162080;
  --border: #1e1e30;
  --border2: #2a2a40;
  --text: #dde1f0;
  --text2: #8891b0;
  --text3: #4a5070;
  --accent: #7c3aed;
  --accent2: #9d5cf7;
  --green: #10b981;
  --yellow: #f59e0b;
  --red: #f43f5e;
  --blue: #3b82f6;
  --cyan: #06b6d4;
  --sidebar: 230px;
  --radius: 8px;
  --font: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; background: var(--bg); color: var(--text); font-family: var(--font); font-size: 13px; }
a { color: var(--accent2); text-decoration: none; }
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border2); border-radius: 3px; }

/* ── Layout ── */
.app { display: flex; height: 100vh; overflow: hidden; }

/* ── Sidebar ── */
.sidebar {
  width: var(--sidebar); min-width: var(--sidebar);
  background: var(--bg2); border-right: 1px solid var(--border);
  display: flex; flex-direction: column; overflow: hidden;
}
.sidebar-logo {
  padding: 18px 16px 12px;
  border-bottom: 1px solid var(--border);
  display: flex; align-items: center; gap: 10px;
}
.sidebar-logo .logo-mark {
  width: 28px; height: 28px; border-radius: 6px;
  background: var(--accent); display: flex; align-items: center; justify-content: center;
  font-weight: 700; font-size: 14px; color: white; flex-shrink: 0;
}
.sidebar-logo .logo-name { font-weight: 600; font-size: 14px; color: var(--text); }
.sidebar-logo .logo-tag { font-size: 10px; color: var(--text2); margin-top: 1px; }

.heartbeat-widget {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  font-size: 11px;
}
.heartbeat-widget .hb-row { display: flex; align-items: center; gap: 6px; margin-bottom: 3px; }
.hb-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--text3); flex-shrink: 0; }
.hb-dot.alive { background: var(--green); animation: pulse 2s infinite; }
.hb-dot.stale { background: var(--red); }
@keyframes pulse { 0%,100% { opacity:1; } 50% { opacity:.4; } }
.hb-phase { color: var(--text2); }
.hb-age { color: var(--text3); font-size: 10px; }

.nav { flex: 1; overflow-y: auto; padding: 8px 0; }
.nav-item {
  display: flex; align-items: center; justify-content: space-between;
  padding: 7px 14px; cursor: pointer; border-radius: 0;
  color: var(--text2); transition: all .15s; gap: 8px;
  border-left: 2px solid transparent;
}
.nav-item:hover { background: var(--card); color: var(--text); }
.nav-item.active { background: var(--card); color: var(--accent2); border-left-color: var(--accent); }
.nav-item .nav-icon { font-size: 14px; flex-shrink: 0; }
.nav-item .nav-label { flex: 1; }
.badge {
  background: var(--accent); color: white; font-size: 10px;
  padding: 1px 5px; border-radius: 9px; line-height: 1.4; min-width: 18px; text-align: center;
}
.badge.warn { background: var(--yellow); color: #1a1000; }
.badge.danger { background: var(--red); }

.sidebar-footer {
  padding: 10px 14px;
  border-top: 1px solid var(--border);
  font-size: 10px; color: var(--text3);
}

/* ── Main ── */
.main { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.topbar {
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  display: flex; align-items: center; justify-content: space-between;
  background: var(--bg2);
}
.topbar h1 { font-size: 16px; font-weight: 600; color: var(--text); }
.topbar-actions { display: flex; gap: 8px; }
.content { flex: 1; overflow-y: auto; padding: 20px; }
.section { display: none; }
.section.active { display: block; }

/* ── Cards & Grids ── */
.stat-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 12px; margin-bottom: 20px; }
.stat-card {
  background: var(--card); border: 1px solid var(--border); border-radius: var(--radius);
  padding: 14px 16px;
}
.stat-card .label { font-size: 10px; color: var(--text2); text-transform: uppercase; letter-spacing: .05em; margin-bottom: 6px; }
.stat-card .value { font-size: 22px; font-weight: 700; color: var(--text); line-height: 1; }
.stat-card .sub { font-size: 10px; color: var(--text2); margin-top: 4px; }
.stat-card.green { border-color: #10b98140; }
.stat-card.green .value { color: var(--green); }
.stat-card.yellow { border-color: #f59e0b40; }
.stat-card.yellow .value { color: var(--yellow); }
.stat-card.red { border-color: #f43f5e40; }
.stat-card.red .value { color: var(--red); }
.stat-card.blue { border-color: #3b82f640; }
.stat-card.blue .value { color: var(--blue); }

/* ── Panels ── */
.panel {
  background: var(--card); border: 1px solid var(--border); border-radius: var(--radius);
  margin-bottom: 16px;
}
.panel-header {
  padding: 10px 16px; border-bottom: 1px solid var(--border);
  display: flex; align-items: center; justify-content: space-between;
  font-size: 12px; font-weight: 600; color: var(--text2); text-transform: uppercase; letter-spacing: .05em;
}
.panel-body { padding: 14px 16px; }

/* ── Tables ── */
table { width: 100%; border-collapse: collapse; font-size: 12px; }
th { text-align: left; padding: 8px 10px; color: var(--text2); font-weight: 500; font-size: 10px; text-transform: uppercase; letter-spacing: .05em; border-bottom: 1px solid var(--border); }
td { padding: 8px 10px; border-bottom: 1px solid var(--border); color: var(--text); vertical-align: top; }
tr:last-child td { border-bottom: none; }
tr:hover td { background: var(--card2); }
.td-mono { font-family: var(--font); font-size: 11px; }
.td-trunc { max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.td-wrap { word-break: break-word; max-width: 400px; }

/* ── Badges ── */
.tag {
  display: inline-block; padding: 2px 7px; border-radius: 4px;
  font-size: 10px; font-weight: 600; text-transform: uppercase;
}
.tag-pending { background: #f59e0b22; color: var(--yellow); border: 1px solid #f59e0b44; }
.tag-approved { background: #10b98122; color: var(--green); border: 1px solid #10b98144; }
.tag-rejected { background: #f43f5e22; color: var(--red); border: 1px solid #f43f5e44; }
.tag-executed { background: #3b82f622; color: var(--blue); border: 1px solid #3b82f644; }
.tag-proposed { background: #7c3aed22; color: var(--accent2); border: 1px solid #7c3aed44; }
.tag-applied { background: #10b98122; color: var(--green); border: 1px solid #10b98144; }
.tag-passed { background: #10b98122; color: var(--green); border: 1px solid #10b98144; }
.tag-failed { background: #f43f5e22; color: var(--red); border: 1px solid #f43f5e44; }
.tag-shell { background: #06b6d422; color: var(--cyan); border: 1px solid #06b6d444; }
.tag-http { background: #3b82f622; color: var(--blue); border: 1px solid #3b82f644; }
.tag-internal { background: #7c3aed22; color: var(--accent2); border: 1px solid #7c3aed44; }
.tag-hot { background: #f59e0b22; color: var(--yellow); }
.tag-cold { background: #3b82f622; color: var(--blue); }
.tag-goal-open { background: #7c3aed22; color: var(--accent2); }
.tag-goal-done { background: #10b98122; color: var(--green); }

/* ── Buttons ── */
.btn {
  display: inline-flex; align-items: center; gap: 5px;
  padding: 6px 12px; border-radius: 5px; border: 1px solid var(--border2);
  background: var(--card); color: var(--text); cursor: pointer;
  font-family: var(--font); font-size: 12px; transition: all .15s;
}
.btn:hover { border-color: var(--accent); color: var(--accent2); }
.btn-primary { background: var(--accent); border-color: var(--accent); color: white; }
.btn-primary:hover { background: var(--accent2); border-color: var(--accent2); color: white; }
.btn-sm { padding: 3px 8px; font-size: 11px; }
.btn-danger { border-color: #f43f5e44; color: var(--red); }
.btn-danger:hover { background: #f43f5e22; border-color: var(--red); }
.btn-success { border-color: #10b98144; color: var(--green); }
.btn-success:hover { background: #10b98122; border-color: var(--green); }

/* ── Events feed ── */
.event-feed {
  background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius);
  height: 280px; overflow-y: auto; padding: 10px 12px;
  font-size: 11px; line-height: 1.6;
}
.event-line { display: flex; gap: 8px; margin-bottom: 2px; }
.event-kind { color: var(--accent2); flex-shrink: 0; min-width: 200px; }
.event-detail { color: var(--text2); word-break: break-word; }

/* ── Forms ── */
.form-row { display: flex; gap: 8px; margin-bottom: 12px; }
.form-input {
  flex: 1; padding: 7px 10px; background: var(--bg); border: 1px solid var(--border);
  border-radius: 5px; color: var(--text); font-family: var(--font); font-size: 12px;
}
.form-input:focus { outline: none; border-color: var(--accent); }
textarea.form-input { resize: vertical; min-height: 300px; }

/* ── Tabs ── */
.tabs { display: flex; gap: 0; border-bottom: 1px solid var(--border); margin-bottom: 16px; }
.tab {
  padding: 8px 16px; cursor: pointer; color: var(--text2); font-size: 12px;
  border-bottom: 2px solid transparent; margin-bottom: -1px; transition: all .15s;
}
.tab.active { color: var(--accent2); border-bottom-color: var(--accent); }
.tab:hover:not(.active) { color: var(--text); }

/* ── Two-col layout ── */
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 900px) { .two-col { grid-template-columns: 1fr; } }

/* ── Identity editor ── */
.identity-tabs { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 12px; }
.identity-tab { padding: 5px 10px; border-radius: 4px; cursor: pointer; font-size: 11px; color: var(--text2); border: 1px solid transparent; }
.identity-tab.active { background: var(--card); border-color: var(--border2); color: var(--accent2); }
.identity-tab:hover:not(.active) { color: var(--text); }

/* ── Toast ── */
.toast {
  position: fixed; bottom: 20px; right: 20px; z-index: 1000;
  background: var(--card); border: 1px solid var(--border2); border-radius: 6px;
  padding: 10px 16px; font-size: 12px; color: var(--text);
  box-shadow: 0 4px 20px #00000080; opacity: 0; transform: translateY(8px);
  transition: all .2s; pointer-events: none;
}
.toast.show { opacity: 1; transform: translateY(0); }
.toast.success { border-color: #10b98160; color: var(--green); }
.toast.error { border-color: #f43f5e60; color: var(--red); }

/* ── Loading spinner ── */
.loading { color: var(--text3); padding: 20px; text-align: center; }

/* ── Empty state ── */
.empty { color: var(--text3); padding: 30px; text-align: center; font-size: 12px; }

/* ── Memory item ── */
.memory-item {
  padding: 10px; border-bottom: 1px solid var(--border); display: flex;
  align-items: flex-start; gap: 10px;
}
.memory-item:last-child { border-bottom: none; }
.memory-meta { flex: 1; min-width: 0; }
.memory-content { color: var(--text); font-size: 12px; word-break: break-word; margin-bottom: 4px; }
.memory-tags { display: flex; gap: 4px; flex-wrap: wrap; }
.memory-tag { font-size: 10px; color: var(--text2); background: var(--bg); padding: 1px 5px; border-radius: 3px; }
.memory-score { font-size: 10px; color: var(--text3); }
.memory-actions { display: flex; gap: 4px; flex-shrink: 0; }

/* ── Score chart bars ── */
.score-bar { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.score-bar-label { font-size: 10px; color: var(--text2); width: 110px; flex-shrink: 0; text-align: right; }
.score-bar-track { flex: 1; height: 5px; background: var(--border); border-radius: 3px; }
.score-bar-fill { height: 100%; border-radius: 3px; background: var(--accent); transition: width .3s; }
.score-bar-val { font-size: 10px; color: var(--text2); width: 35px; }

/* ── Evolution proposal ── */
.proposal-card {
  border: 1px solid var(--border); border-radius: var(--radius);
  margin-bottom: 12px; padding: 14px 16px;
  background: var(--card);
}
.proposal-header { display: flex; align-items: center; gap: 8px; margin-bottom: 8px; }
.proposal-title { font-size: 13px; font-weight: 600; flex: 1; }
.proposal-body { font-size: 12px; color: var(--text2); margin-bottom: 8px; }
.proposal-detail { font-family: var(--font); font-size: 11px; background: var(--bg); padding: 8px 10px; border-radius: 4px; color: var(--text); white-space: pre-wrap; word-break: break-word; margin-bottom: 8px; }

/* ── Delegation link ── */
.link-card {
  border: 1px solid var(--border); border-radius: var(--radius);
  padding: 12px 14px; margin-bottom: 8px; background: var(--card);
}
.link-header { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
.link-name { font-weight: 600; font-size: 13px; }
.link-meta { font-size: 11px; color: var(--text2); }
</style>
</head>
<body>
<div class="app">

<!-- ── Sidebar ── -->
<nav class="sidebar">
  <div class="sidebar-logo">
    <div class="logo-mark">P</div>
    <div>
      <div class="logo-name" id="instance-name">Praxis</div>
      <div class="logo-tag" id="instance-backend">loading…</div>
    </div>
  </div>

  <div class="heartbeat-widget">
    <div class="hb-row">
      <div class="hb-dot" id="hb-dot"></div>
      <span class="hb-phase" id="hb-phase">—</span>
    </div>
    <div class="hb-age" id="hb-age">heartbeat unknown</div>
  </div>

  <div class="nav" id="nav">
    <div class="nav-item active" data-section="overview" onclick="navigate('overview')">
      <span class="nav-icon">◈</span><span class="nav-label">Overview</span>
    </div>
    <div class="nav-item" data-section="sessions" onclick="navigate('sessions')">
      <span class="nav-icon">⟳</span><span class="nav-label">Sessions</span>
    </div>
    <div class="nav-item" data-section="approvals" onclick="navigate('approvals')">
      <span class="nav-icon">✓</span><span class="nav-label">Approvals</span>
      <span class="badge danger" id="nav-approvals-badge" style="display:none">0</span>
    </div>
    <div class="nav-item" data-section="memory" onclick="navigate('memory')">
      <span class="nav-icon">◎</span><span class="nav-label">Memory</span>
    </div>
    <div class="nav-item" data-section="tools" onclick="navigate('tools')">
      <span class="nav-icon">⚙</span><span class="nav-label">Tools</span>
    </div>
    <div class="nav-item" data-section="goals" onclick="navigate('goals')">
      <span class="nav-icon">◇</span><span class="nav-label">Goals</span>
    </div>
    <div class="nav-item" data-section="identity" onclick="navigate('identity')">
      <span class="nav-icon">⊡</span><span class="nav-label">Identity</span>
    </div>
    <div class="nav-item" data-section="system" onclick="navigate('system')">
      <span class="nav-icon">≡</span><span class="nav-label">System</span>
    </div>
  </div>

  <div class="sidebar-footer" id="sidebar-footer">Praxis</div>
</nav>

<!-- ── Main ── -->
<div class="main">
  <div class="topbar">
    <h1 id="topbar-title">Overview</h1>
    <div class="topbar-actions" id="topbar-actions"></div>
  </div>
  <div class="content">

    <!-- ── Overview ── -->
    <div class="section active" id="section-overview">
      <div class="stat-grid" id="overview-stats"></div>
      <div class="two-col">
        <div>
          <div class="panel">
            <div class="panel-header">
              Live Events
              <button class="btn btn-sm" onclick="clearFeed()">Clear</button>
            </div>
            <div class="event-feed" id="event-feed"></div>
          </div>
        </div>
        <div>
          <div class="panel">
            <div class="panel-header">Quick Controls</div>
            <div class="panel-body">
              <div class="form-row" style="margin-bottom:10px">
                <input class="form-input" id="ctrl-task" placeholder="Optional task description…">
                <button class="btn btn-primary" onclick="doRun()">▶ Run</button>
              </div>
              <div class="form-row">
                <input class="form-input" id="ctrl-wake-task" placeholder="Optional steering task…">
                <button class="btn" onclick="doWake(false)">Wake</button>
                <button class="btn btn-danger btn-sm" onclick="doWake(true)">Urgent</button>
              </div>
            </div>
          </div>
          <div class="panel">
            <div class="panel-header">Last Session</div>
            <div class="panel-body" id="overview-last-session"><div class="loading">loading…</div></div>
          </div>
          <div class="panel">
            <div class="panel-header">Score (last 5)</div>
            <div class="panel-body" id="overview-scores"><div class="loading">loading…</div></div>
          </div>
        </div>
      </div>
    </div>

    <!-- ── Sessions ── -->
    <div class="section" id="section-sessions">
      <div class="panel">
        <div class="panel-header">Recent Sessions (last 50)<span id="session-count" style="color:var(--text3);font-weight:400"></span></div>
        <div style="overflow-x:auto">
          <table>
            <thead>
              <tr><th>#</th><th>Outcome</th><th>Goal</th><th>Task</th><th>Action Summary</th><th>Started</th><th>Duration</th></tr>
            </thead>
            <tbody id="sessions-tbody"><tr><td colspan="7" class="loading">loading…</td></tr></tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- ── Approvals ── -->
    <div class="section" id="section-approvals">
      <div class="tabs">
        <div class="tab active" data-tab="pending" onclick="setApprovalsTab('pending')">Pending</div>
        <div class="tab" data-tab="all" onclick="setApprovalsTab('all')">All</div>
      </div>
      <div id="approvals-list"><div class="loading">loading…</div></div>
    </div>

    <!-- ── Memory ── -->
    <div class="section" id="section-memory">
      <div style="display:flex;gap:8px;margin-bottom:16px;align-items:center">
        <input class="form-input" style="max-width:260px" id="memory-filter" placeholder="Filter memories…" oninput="filterMemories()">
        <button class="btn" onclick="consolidateMemories()">⇌ Consolidate</button>
        <span id="memory-counts" style="color:var(--text2);font-size:11px"></span>
      </div>
      <div class="tabs">
        <div class="tab active" data-tab="hot" onclick="setMemoryTab('hot')">Hot</div>
        <div class="tab" data-tab="cold" onclick="setMemoryTab('cold')">Cold</div>
      </div>
      <div class="panel">
        <div id="memory-list"><div class="loading">loading…</div></div>
      </div>
    </div>

    <!-- ── Tools ── -->
    <div class="section" id="section-tools">
      <div class="panel">
        <div class="panel-header">Registered Tools <span id="tool-count" style="color:var(--text3);font-weight:400"></span></div>
        <div style="overflow-x:auto">
          <table>
            <thead><tr><th>Name</th><th>Kind</th><th>Level</th><th>Approval</th><th>Description</th></tr></thead>
            <tbody id="tools-tbody"><tr><td colspan="5" class="loading">loading…</td></tr></tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- ── Goals ── -->
    <div class="section" id="section-goals">
      <div class="panel" style="margin-bottom:16px">
        <div class="panel-header">Add Goal</div>
        <div class="panel-body">
          <div class="form-row">
            <input class="form-input" id="goal-input" placeholder="Describe the new goal…" onkeydown="if(event.key==='Enter')addGoal()">
            <button class="btn btn-primary" onclick="addGoal()">+ Add</button>
          </div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-header">Goals <span id="goals-count" style="color:var(--text3);font-weight:400"></span></div>
        <div id="goals-list"><div class="loading">loading…</div></div>
      </div>
    </div>

    <!-- ── Identity ── -->
    <div class="section" id="section-identity">
      <div class="identity-tabs" id="identity-tabs">
        <div class="identity-tab active" data-file="identity" onclick="loadIdentityFile('identity')">IDENTITY.md</div>
        <div class="identity-tab" data-file="soul" onclick="loadIdentityFile('soul')">SOUL.md</div>
        <div class="identity-tab" data-file="agents" onclick="loadIdentityFile('agents')">AGENTS.md</div>
        <div class="identity-tab" data-file="goals" onclick="loadIdentityFile('goals')">GOALS.md</div>
        <div class="identity-tab" data-file="journal" onclick="loadIdentityFile('journal')">JOURNAL.md</div>
        <div class="identity-tab" data-file="patterns" onclick="loadIdentityFile('patterns')">PATTERNS.md</div>
        <div class="identity-tab" data-file="learnings" onclick="loadIdentityFile('learnings')">LEARNINGS.md</div>
        <div class="identity-tab" data-file="roadmap" onclick="loadIdentityFile('roadmap')">ROADMAP.md</div>
      </div>
      <div class="panel">
        <div class="panel-header" id="identity-file-header">—
          <div style="display:flex;gap:6px">
            <span id="identity-status" style="font-size:10px;color:var(--text3);font-weight:400"></span>
            <button class="btn btn-sm btn-primary" id="identity-save-btn" onclick="saveIdentityFile()" style="display:none">Save</button>
          </div>
        </div>
        <div class="panel-body" style="padding:0">
          <textarea class="form-input" id="identity-editor" style="border:none;border-radius:0;min-height:500px;background:var(--bg);" oninput="markIdentityDirty()"></textarea>
        </div>
      </div>
    </div>

    <!-- ── System ── -->
    <div class="section" id="section-system">
      <div class="two-col">
        <div>
          <div class="panel">
            <div class="panel-header">Heartbeat</div>
            <div class="panel-body" id="system-heartbeat"><div class="loading">loading…</div></div>
          </div>
          <div class="panel">
            <div class="panel-header">Canary Ledger</div>
            <div class="panel-body" id="system-canary"><div class="loading">loading…</div></div>
          </div>
          <div class="panel">
            <div class="panel-header">Delegation Links</div>
            <div class="panel-body" id="system-delegation"><div class="loading">loading…</div></div>
          </div>
        </div>
        <div>
          <div class="panel">
            <div class="panel-header">Evolution Proposals</div>
            <div id="system-evolution"><div class="loading">loading…</div></div>
          </div>
        </div>
      </div>
      <div class="panel">
        <div class="panel-header">Configuration</div>
        <div class="panel-body" id="system-config" style="overflow-x:auto"><div class="loading">loading…</div></div>
      </div>
    </div>

  </div><!-- /content -->
</div><!-- /main -->
</div><!-- /app -->

<div class="toast" id="toast"></div>

<script>
// ── State ──────────────────────────────────────────────────────────────────────
const S = {
  section: 'overview',
  summary: null,
  memoryTab: 'hot',
  approvalsTab: 'pending',
  memoryHot: [],
  memoryCold: [],
  identityFile: 'identity',
  identityDirty: false,
};

// ── Utilities ─────────────────────────────────────────────────────────────────
async function api(path, opts = {}) {
  const r = await fetch('/api' + path, { headers: { 'Content-Type': 'application/json' }, ...opts });
  if (!r.ok) { const t = await r.text(); throw new Error(t || r.statusText); }
  return r.json();
}
function toast(msg, type = '') {
  const el = document.getElementById('toast');
  el.textContent = msg;
  el.className = 'toast show' + (type ? ' ' + type : '');
  clearTimeout(el._t);
  el._t = setTimeout(() => el.className = 'toast', 2800);
}
function fmt(val, digits = 2) {
  return (typeof val === 'number') ? val.toFixed(digits) : String(val ?? '—');
}
function elapsed(isoStr) {
  if (!isoStr) return '—';
  const d = (Date.now() - new Date(isoStr)) / 1000;
  if (d < 60) return Math.round(d) + 's ago';
  if (d < 3600) return Math.round(d/60) + 'm ago';
  if (d < 86400) return Math.round(d/3600) + 'h ago';
  return Math.round(d/86400) + 'd ago';
}
function duration(a, b) {
  if (!a || !b) return '—';
  const d = (new Date(b) - new Date(a)) / 1000;
  return d < 60 ? Math.round(d) + 's' : Math.round(d/60) + 'm';
}
function outcomeTag(o) {
  const map = {
    goal_selected: 'blue', task_selected: 'blue', tool_executed: 'green',
    stop_condition_met: 'approved', delegated: 'proposed', steered: 'pending',
    budget_exhausted: 'rejected', blocked_loop_guard: 'rejected',
    idle: '', review_failed: 'rejected', eval_failed: 'rejected',
  };
  const cl = map[o] || '';
  return `<span class="tag tag-${cl || 'pending'}">${o || 'idle'}</span>`;
}
function setHTML(id, html) { document.getElementById(id).innerHTML = html; }

// ── Navigation ────────────────────────────────────────────────────────────────
function navigate(section) {
  S.section = section;
  document.querySelectorAll('.section').forEach(el => el.classList.toggle('active', el.id === 'section-' + section));
  document.querySelectorAll('.nav-item').forEach(el => el.classList.toggle('active', el.dataset.section === section));
  document.getElementById('topbar-title').textContent = section.charAt(0).toUpperCase() + section.slice(1);
  setupTopbarActions(section);
  loadSection(section);
  history.replaceState(null, '', '#' + section);
}

function setupTopbarActions(section) {
  const el = document.getElementById('topbar-actions');
  el.innerHTML = '';
  if (section === 'memory') {
    el.innerHTML = '<button class="btn" onclick="loadMemory()">↺ Refresh</button>';
  } else if (section === 'approvals') {
    el.innerHTML = '<button class="btn" onclick="loadApprovals()">↺ Refresh</button>';
  } else if (section === 'sessions') {
    el.innerHTML = '<button class="btn" onclick="loadSessions()">↺ Refresh</button>';
  } else if (section === 'system') {
    el.innerHTML = '<button class="btn" onclick="loadSystem()">↺ Refresh</button>';
  }
}

async function loadSection(section) {
  switch(section) {
    case 'overview': return loadOverview();
    case 'sessions': return loadSessions();
    case 'approvals': return loadApprovals();
    case 'memory': return loadMemory();
    case 'tools': return loadTools();
    case 'goals': return loadGoals();
    case 'identity': return loadIdentityFile(S.identityFile);
    case 'system': return loadSystem();
  }
}

// ── Overview ──────────────────────────────────────────────────────────────────
async function loadOverview() {
  try {
    const s = await api('/summary');
    S.summary = s;
    renderOverviewStats(s);
    renderLastSession(s);
    updateSidebarMeta(s);
  } catch(e) { toast(e.message, 'error'); }
  loadScores();
}

function renderOverviewStats(s) {
  const pending = s.pending_approvals || 0;
  const cards = [
    { label: 'Phase', value: s.phase || '—', cls: '', sub: s.last_outcome || '' },
    { label: 'Pending Approvals', value: pending, cls: pending > 0 ? 'yellow' : '', sub: 'tool approvals' },
    { label: 'Hot Memories', value: s.operational_memory?.do_not_repeat !== undefined ? '—' : '—', cls: 'blue', sub: 'session store' },
    { label: 'Sessions', value: s.last_session?.session_num || '—', cls: '', sub: 'day ' + (s.last_session?.session_num ? '—' : '') },
    { label: 'Registered Tools', value: s.registered_tools || '—', cls: 'green', sub: 'tools active' },
    { label: 'Tokens (last)', value: s.last_token_summary ? fmt(s.last_token_summary.tokens_used, 0) : '—', cls: '', sub: s.last_token_summary ? '$' + (s.last_token_summary.estimated_cost_micros/1e6).toFixed(4) : '' },
    { label: 'Drift Status', value: s.drift_status || '—', cls: s.drift_status === 'stable' ? 'green' : 'yellow', sub: 'argus analysis' },
    { label: 'Anatomy', value: s.anatomy_entries || '—', cls: '', sub: 'entries indexed' },
  ];
  document.getElementById('overview-stats').innerHTML = cards.map(c =>
    `<div class="stat-card ${c.cls}">
      <div class="label">${c.label}</div>
      <div class="value">${c.value}</div>
      ${c.sub ? `<div class="sub">${c.sub}</div>` : ''}
    </div>`
  ).join('');

  // Update approvals badge
  const badge = document.getElementById('nav-approvals-badge');
  badge.style.display = pending > 0 ? 'inline-block' : 'none';
  badge.textContent = pending;
}

function renderLastSession(s) {
  const ls = s.last_session;
  if (!ls) { setHTML('overview-last-session', '<div class="empty">No sessions yet</div>'); return; }
  setHTML('overview-last-session', `
    <div style="font-size:12px">
      <div style="margin-bottom:6px">${outcomeTag(ls.outcome)} <span style="color:var(--text2);font-size:11px">session #${ls.session_num}</span></div>
      <div style="color:var(--text2);font-size:11px">${elapsed(ls.ended_at)}</div>
      ${s.last_review_status ? `<div style="margin-top:6px">Review: <span class="tag tag-${s.last_review_status}">${s.last_review_status}</span></div>` : ''}
    </div>
  `);
}

async function loadScores() {
  try {
    const scores = await api('/score');
    if (!scores.length) { setHTML('overview-scores', '<div class="empty">No scores yet</div>'); return; }
    const last5 = scores.slice(-5);
    const dims = ['anticipation', 'follow_through', 'reliability', 'independence', 'composite'];
    const avgs = {};
    dims.forEach(d => { avgs[d] = last5.reduce((a, s) => a + (s[d] || 0), 0) / last5.length; });
    setHTML('overview-scores', dims.map(d => `
      <div class="score-bar">
        <div class="score-bar-label">${d.replace('_', ' ')}</div>
        <div class="score-bar-track"><div class="score-bar-fill" style="width:${(avgs[d]*100).toFixed(0)}%"></div></div>
        <div class="score-bar-val">${(avgs[d]*100).toFixed(0)}%</div>
      </div>
    `).join(''));
  } catch(e) { setHTML('overview-scores', '<div class="empty">—</div>'); }
}

function updateSidebarMeta(s) {
  document.getElementById('instance-name').textContent = s.instance_name || 'Praxis';
  document.getElementById('instance-backend').textContent = s.backend || '—';
  document.getElementById('sidebar-footer').textContent = 'data: ' + (s.data_dir || '—').split('/').slice(-2).join('/');
}

// ── Event Feed (SSE) ──────────────────────────────────────────────────────────
function startEventStream() {
  const feed = document.getElementById('event-feed');
  // Seed with recent events
  fetch('/events/recent').then(r=>r.json()).then(items => {
    items.forEach(e => appendEvent(feed, e.kind, e.detail));
  }).catch(()=>{});

  const es = new EventSource('/events');
  es.onmessage = ev => appendEvent(feed, ev.type || '—', ev.data);
  es.addEventListener('error', () => {});
  // Also listen for named events
  const kinds = ['agent:orient_start','agent:decide_start','agent:act_start','agent:reflect_start',
    'agent:wake_intent_consumed','agent:tool_call','agent:delegated','agent:steered',
    'agent:memory_consolidated','agent:learning_opportunities_found','agent:loop_guard_triggered'];
  kinds.forEach(k => es.addEventListener(k, ev => appendEvent(feed, k, ev.data)));
}

function appendEvent(feed, kind, detail) {
  const line = document.createElement('div');
  line.className = 'event-line';
  line.innerHTML = `<span class="event-kind">${kind}</span><span class="event-detail">${detail}</span>`;
  feed.appendChild(line);
  feed.scrollTop = feed.scrollHeight;
  // Keep max 200 lines
  while (feed.children.length > 200) feed.removeChild(feed.firstChild);
}

function clearFeed() { document.getElementById('event-feed').innerHTML = ''; }

// ── Sessions ──────────────────────────────────────────────────────────────────
async function loadSessions() {
  setHTML('sessions-tbody', '<tr><td colspan="7" class="loading">loading…</td></tr>');
  try {
    const rows = await api('/sessions');
    document.getElementById('session-count').textContent = ' — ' + rows.length + ' shown';
    if (!rows.length) { setHTML('sessions-tbody', '<tr><td colspan="7" class="empty">No sessions</td></tr>'); return; }
    document.getElementById('sessions-tbody').innerHTML = rows.map(r => `
      <tr>
        <td class="td-mono">${r.session_num}</td>
        <td>${outcomeTag(r.outcome)}</td>
        <td class="td-trunc" style="max-width:140px" title="${r.selected_goal_title||''}">${r.selected_goal_id ? `<span style="color:var(--text2)">${r.selected_goal_id}</span>` : '<span style="color:var(--text3)">—</span>'}</td>
        <td class="td-trunc" style="max-width:120px">${r.selected_task ? `<span title="${r.selected_task}">${r.selected_task}</span>` : '<span style="color:var(--text3)">—</span>'}</td>
        <td class="td-wrap" style="max-width:300px;font-size:11px;color:var(--text2)">${r.action_summary || '—'}</td>
        <td class="td-mono" style="white-space:nowrap;color:var(--text2);font-size:11px">${elapsed(r.started_at)}</td>
        <td class="td-mono" style="color:var(--text3)">${duration(r.started_at, r.ended_at)}</td>
      </tr>
    `).join('');
  } catch(e) { setHTML('sessions-tbody', `<tr><td colspan="7" class="empty">${e.message}</td></tr>`); }
}

// ── Approvals ─────────────────────────────────────────────────────────────────
let allApprovals = [];
async function loadApprovals() {
  setHTML('approvals-list', '<div class="loading">loading…</div>');
  try {
    allApprovals = await api('/approvals');
    renderApprovals();
  } catch(e) { setHTML('approvals-list', `<div class="empty">${e.message}</div>`); }
}

function setApprovalsTab(tab) {
  S.approvalsTab = tab;
  document.querySelectorAll('#section-approvals .tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
  renderApprovals();
}

function renderApprovals() {
  const list = S.approvalsTab === 'pending' ? allApprovals.filter(a => a.status === 'pending') : allApprovals;
  if (!list.length) { setHTML('approvals-list', '<div class="empty">No approvals</div>'); return; }
  document.getElementById('approvals-list').innerHTML = list.map(a => `
    <div class="panel" style="margin-bottom:10px">
      <div class="panel-header">
        #${a.id} — ${a.tool_name}
        <div style="display:flex;gap:6px;align-items:center">
          <span class="tag tag-${a.status}">${a.status}</span>
          ${a.status === 'pending' ? `
            <button class="btn btn-sm btn-success" onclick="approveItem(${a.id})">✓ Approve</button>
            <button class="btn btn-sm btn-danger" onclick="rejectItem(${a.id})">✗ Reject</button>
          ` : ''}
        </div>
      </div>
      <div class="panel-body">
        <div style="margin-bottom:6px;font-size:12px">${a.summary}</div>
        <div style="font-size:11px;color:var(--text2)">by <strong>${a.requested_by}</strong> · ${elapsed(a.created_at)}</div>
        ${a.write_paths?.length ? `<div style="font-size:11px;color:var(--text3);margin-top:4px">write: ${a.write_paths.join(', ')}</div>` : ''}
        ${a.payload_json ? `<div style="margin-top:8px"><code style="font-size:10px;color:var(--text2)">${a.payload_json}</code></div>` : ''}
        ${a.status_note ? `<div style="font-size:11px;color:var(--yellow);margin-top:4px">Note: ${a.status_note}</div>` : ''}
      </div>
    </div>
  `).join('');
}

async function approveItem(id) {
  try {
    await api(`/approvals/${id}/approve`, { method: 'POST' });
    toast('Approved #' + id, 'success');
    loadApprovals();
  } catch(e) { toast(e.message, 'error'); }
}
async function rejectItem(id) {
  try {
    await api(`/approvals/${id}/reject`, { method: 'POST' });
    toast('Rejected #' + id);
    loadApprovals();
  } catch(e) { toast(e.message, 'error'); }
}

// ── Memory ────────────────────────────────────────────────────────────────────
async function loadMemory() {
  setHTML('memory-list', '<div class="loading">loading…</div>');
  try {
    const [hot, cold] = await Promise.all([api('/memories/hot'), api('/memories/cold')]);
    S.memoryHot = hot;
    S.memoryCold = cold;
    document.getElementById('memory-counts').textContent = `${hot.length} hot · ${cold.length} cold`;
    renderMemories();
  } catch(e) { setHTML('memory-list', `<div class="empty">${e.message}</div>`); }
}

function setMemoryTab(tab) {
  S.memoryTab = tab;
  document.querySelectorAll('#section-memory .tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
  renderMemories();
}

function filterMemories() { renderMemories(); }

function renderMemories() {
  const q = document.getElementById('memory-filter').value.toLowerCase();
  const list = (S.memoryTab === 'hot' ? S.memoryHot : S.memoryCold)
    .filter(m => !q || m.content.toLowerCase().includes(q) || (m.summary||'').toLowerCase().includes(q) || m.tags.join(' ').toLowerCase().includes(q));
  if (!list.length) { setHTML('memory-list', '<div class="empty">No memories</div>'); return; }
  document.getElementById('memory-list').innerHTML = list.map(m => `
    <div class="memory-item">
      <div class="memory-meta">
        <div class="memory-content">${m.content.length > 200 ? m.content.slice(0,200)+'…' : m.content}</div>
        ${m.summary ? `<div style="font-size:11px;color:var(--text2);margin-bottom:4px">${m.summary}</div>` : ''}
        <div style="display:flex;gap:8px;align-items:center">
          <span class="memory-score">score: ${fmt(m.score)}</span>
          <span class="tag tag-${m.tier}">${m.tier}</span>
          <span class="tag" style="font-size:9px;background:var(--bg);color:var(--text3)">${m.memory_type}</span>
          ${m.tags.map(t => `<span class="memory-tag">${t}</span>`).join('')}
        </div>
      </div>
      <div class="memory-actions">
        ${m.tier === 'hot' ? `<button class="btn btn-sm btn-success" onclick="reinforceMemory(${m.id})" title="Reinforce">↑</button>` : ''}
        <button class="btn btn-sm btn-danger" onclick="forgetMemory(${m.id})" title="Forget">✗</button>
      </div>
    </div>
  `).join('');
}

async function reinforceMemory(id) {
  try {
    await api(`/memories/${id}/reinforce`, { method: 'POST' });
    toast('Memory reinforced', 'success');
    loadMemory();
  } catch(e) { toast(e.message, 'error'); }
}
async function forgetMemory(id) {
  if (!confirm('Forget this memory?')) return;
  try {
    await api(`/memories/${id}/forget`, { method: 'POST' });
    toast('Memory forgotten');
    loadMemory();
  } catch(e) { toast(e.message, 'error'); }
}
async function consolidateMemories() {
  try {
    const r = await api('/memories/consolidate', { method: 'POST' });
    toast(`Consolidated ${r.consolidated}, pruned ${r.pruned}`, 'success');
    loadMemory();
  } catch(e) { toast(e.message, 'error'); }
}

// ── Tools ─────────────────────────────────────────────────────────────────────
async function loadTools() {
  setHTML('tools-tbody', '<tr><td colspan="5" class="loading">loading…</td></tr>');
  try {
    const tools = await api('/tools');
    document.getElementById('tool-count').textContent = ' — ' + tools.length;
    if (!tools.length) { setHTML('tools-tbody', '<tr><td colspan="5" class="empty">No tools</td></tr>'); return; }
    document.getElementById('tools-tbody').innerHTML = tools.map(t => `
      <tr>
        <td style="font-weight:600">${t.name}</td>
        <td><span class="tag tag-${t.kind}">${t.kind}</span></td>
        <td class="td-mono">${t.required_level}</td>
        <td>${t.requires_approval ? '<span class="tag tag-pending">required</span>' : '<span style="color:var(--text3)">auto</span>'}</td>
        <td style="color:var(--text2);font-size:11px">${t.description || '—'}</td>
      </tr>
    `).join('');
  } catch(e) { setHTML('tools-tbody', `<tr><td colspan="5" class="empty">${e.message}</td></tr>`); }
}

// ── Goals ─────────────────────────────────────────────────────────────────────
async function loadGoals() {
  setHTML('goals-list', '<div class="loading">loading…</div>');
  try {
    const goals = await api('/goals');
    const open = goals.filter(g => !g.completed).length;
    document.getElementById('goals-count').textContent = ` — ${open} open, ${goals.length - open} done`;
    if (!goals.length) { setHTML('goals-list', '<div class="empty">No goals</div>'); return; }
    document.getElementById('goals-list').innerHTML = goals.map(g => `
      <div style="display:flex;align-items:flex-start;gap:10px;padding:10px 14px;border-bottom:1px solid var(--border)">
        <span class="tag ${g.completed ? 'tag-goal-done' : 'tag-goal-open'}" style="margin-top:1px">${g.completed ? 'done' : 'open'}</span>
        <div style="flex:1">
          ${g.raw_id ? `<span style="color:var(--text2);font-size:11px;margin-right:6px">${g.raw_id}</span>` : ''}
          <span style="color:${g.completed ? 'var(--text3)' : 'var(--text)'}${g.completed ? ';text-decoration:line-through' : ''}">${g.title}</span>
        </div>
      </div>
    `).join('');
  } catch(e) { setHTML('goals-list', `<div class="empty">${e.message}</div>`); }
}

async function addGoal() {
  const desc = document.getElementById('goal-input').value.trim();
  if (!desc) return;
  try {
    const r = await api('/goals', { method: 'POST', body: JSON.stringify({ description: desc }) });
    toast(`Added goal ${r.id}`, 'success');
    document.getElementById('goal-input').value = '';
    loadGoals();
  } catch(e) { toast(e.message, 'error'); }
}

// ── Identity ──────────────────────────────────────────────────────────────────
async function loadIdentityFile(file) {
  S.identityFile = file;
  S.identityDirty = false;
  document.querySelectorAll('.identity-tab').forEach(t => t.classList.toggle('active', t.dataset.file === file));
  document.getElementById('identity-file-header').firstChild.textContent = file.toUpperCase() + '.md';
  document.getElementById('identity-status').textContent = '';
  document.getElementById('identity-save-btn').style.display = 'none';
  const editor = document.getElementById('identity-editor');
  editor.value = 'loading…';
  editor.readOnly = true;
  try {
    const r = await api('/identity/' + file);
    editor.value = r.content || '';
    editor.readOnly = !r.writable;
    if (r.writable) {
      document.getElementById('identity-save-btn').style.display = 'inline-flex';
      document.getElementById('identity-status').textContent = 'editable';
    } else {
      document.getElementById('identity-status').textContent = 'read-only';
    }
    S.identityDirty = false;
  } catch(e) { editor.value = 'Error: ' + e.message; }
}

function markIdentityDirty() {
  if (!S.identityDirty) {
    S.identityDirty = true;
    document.getElementById('identity-status').textContent = 'unsaved changes';
  }
}

async function saveIdentityFile() {
  const content = document.getElementById('identity-editor').value;
  try {
    await api('/identity/' + S.identityFile, { method: 'PUT', body: JSON.stringify({ content }) });
    S.identityDirty = false;
    document.getElementById('identity-status').textContent = 'saved';
    toast('Saved ' + S.identityFile, 'success');
  } catch(e) { toast(e.message, 'error'); }
}

// ── System ────────────────────────────────────────────────────────────────────
async function loadSystem() {
  loadHeartbeatPanel();
  loadCanaryPanel();
  loadDelegationPanel();
  loadEvolutionPanel();
  loadConfigPanel();
}

async function loadHeartbeatPanel() {
  try {
    const hb = await api('/heartbeat');
    if (!hb) { setHTML('system-heartbeat', '<div class="empty">No heartbeat</div>'); return; }
    setHTML('system-heartbeat', `
      <div style="display:grid;gap:6px;font-size:12px">
        <div><span style="color:var(--text2)">phase</span> <strong>${hb.phase}</strong></div>
        <div><span style="color:var(--text2)">detail</span> ${hb.detail}</div>
        <div><span style="color:var(--text2)">updated</span> ${elapsed(hb.updated_at)} <span style="color:var(--text3);font-size:10px">(${hb.updated_at})</span></div>
        <div><span style="color:var(--text2)">pid</span> ${hb.pid} · uptime ${Math.round(hb.process_uptime_ms/1000)}s</div>
      </div>
    `);
  } catch(e) { setHTML('system-heartbeat', `<div class="empty">${e.message}</div>`); }
}

async function loadCanaryPanel() {
  try {
    const c = await api('/canary');
    let html = '';
    if (c.frozen.size > 0 || (Array.isArray(c.frozen) ? c.frozen : Object.keys(c.frozen)).length > 0) {
      const frozen = Array.isArray(c.frozen) ? c.frozen : Array.from(c.frozen);
      html += `<div style="margin-bottom:8px"><span style="color:var(--red);font-size:11px">❄ Frozen: ${frozen.join(', ')}</span></div>`;
    }
    if (!c.records?.length) { html += '<div class="empty">No canary records</div>'; }
    else html += c.records.map(r => `
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px;font-size:12px">
        <span class="tag tag-${r.status}">${r.status}</span>
        <strong>${r.provider}/${r.model}</strong>
        <span style="color:var(--text2)">passes: ${r.consecutive_passes}</span>
        <span style="color:var(--text3);font-size:10px">${elapsed(r.checked_at)}</span>
      </div>
    `).join('');
    setHTML('system-canary', html || '<div class="empty">—</div>');
  } catch(e) { setHTML('system-canary', `<div class="empty">${e.message}</div>`); }
}

async function loadDelegationPanel() {
  try {
    const d = await api('/delegation');
    if (!d.links?.length) { setHTML('system-delegation', '<div class="empty">No delegation links</div>'); return; }
    setHTML('system-delegation', d.links.map(l => `
      <div class="link-card">
        <div class="link-header">
          <span class="link-name">${l.name}</span>
          <span class="tag ${l.enabled ? 'tag-approved' : 'tag-rejected'}">${l.enabled ? 'enabled' : 'disabled'}</span>
          <span class="tag" style="background:var(--bg);color:var(--text2)">${l.direction}</span>
        </div>
        <div class="link-meta">${l.endpoint} · concurrency: ${d.active_counts[l.name]||0}/${l.max_concurrency}</div>
      </div>
    `).join(''));
  } catch(e) { setHTML('system-delegation', `<div class="empty">${e.message}</div>`); }
}

async function loadEvolutionPanel() {
  try {
    const proposals = await api('/evolution');
    if (!proposals.length) { setHTML('system-evolution', '<div class="empty" style="padding:20px">No evolution proposals</div>'); return; }
    setHTML('system-evolution', proposals.slice().reverse().map(p => `
      <div class="proposal-card">
        <div class="proposal-header">
          <span class="tag tag-${p.status}">${p.status}</span>
          <span class="tag" style="background:var(--bg);color:var(--text2)">${p.change_kind}</span>
          <span class="proposal-title">${p.title}</span>
          ${p.status === 'proposed' ? `<button class="btn btn-sm btn-success" onclick="approveEvolution('${p.id}')">Approve</button>` : ''}
        </div>
        <div class="proposal-body">${p.motivation}</div>
        <div class="proposal-detail">${p.change_details}</div>
        <div style="font-size:10px;color:var(--text3)">${p.id} · ${elapsed(p.created_at)}</div>
      </div>
    `).join(''));
  } catch(e) { setHTML('system-evolution', `<div class="empty">${e.message}</div>`); }
}

async function approveEvolution(id) {
  try {
    await api(`/evolution/${id}/approve`, { method: 'POST' });
    toast('Evolution approved', 'success');
    loadEvolutionPanel();
  } catch(e) { toast(e.message, 'error'); }
}

async function loadConfigPanel() {
  try {
    const c = await api('/config');
    let html = '';
    for (const [key, val] of Object.entries(c)) {
      if (val) html += `
        <div style="margin-bottom:12px">
          <div style="font-size:10px;color:var(--text2);text-transform:uppercase;margin-bottom:4px">${key}</div>
          <pre style="font-size:11px;color:var(--text);background:var(--bg);padding:10px;border-radius:4px;overflow-x:auto;white-space:pre-wrap;word-break:break-all">${val}</pre>
        </div>
      `;
    }
    setHTML('system-config', html || '<div class="empty">No config files found</div>');
  } catch(e) { setHTML('system-config', `<div class="empty">${e.message}</div>`); }
}

// ── Controls ──────────────────────────────────────────────────────────────────
async function doRun() {
  const task = document.getElementById('ctrl-task').value.trim() || null;
  const btn = event.target;
  btn.disabled = true;
  btn.textContent = 'Running…';
  try {
    const r = await api('/run', { method: 'POST', body: JSON.stringify({ task }) });
    toast('Session complete: ' + r.outcome, 'success');
    loadOverview();
  } catch(e) { toast(e.message, 'error'); }
  finally { btn.disabled = false; btn.textContent = '▶ Run'; }
}

async function doWake(urgent) {
  const task = document.getElementById('ctrl-wake-task').value.trim() || null;
  try {
    await api('/wake', { method: 'POST', body: JSON.stringify({ task, urgent }) });
    toast(urgent ? 'Urgent wake queued' : 'Wake intent queued', 'success');
  } catch(e) { toast(e.message, 'error'); }
}

// ── Heartbeat sidebar widget ───────────────────────────────────────────────────
async function updateHeartbeatWidget() {
  try {
    const hb = await api('/heartbeat');
    const dot = document.getElementById('hb-dot');
    const phase = document.getElementById('hb-phase');
    const age = document.getElementById('hb-age');
    if (!hb) { dot.className = 'hb-dot'; phase.textContent = '—'; age.textContent = 'no heartbeat'; return; }
    const ageSec = (Date.now() - new Date(hb.updated_at)) / 1000;
    const stale = ageSec > 900;
    dot.className = 'hb-dot ' + (stale ? 'stale' : 'alive');
    phase.textContent = hb.phase;
    age.textContent = elapsed(hb.updated_at);
  } catch(e) { document.getElementById('hb-dot').className = 'hb-dot'; }
}

// ── Init ──────────────────────────────────────────────────────────────────────
(function init() {
  const hash = location.hash.replace('#', '') || 'overview';
  navigate(hash);
  startEventStream();
  updateHeartbeatWidget();
  setInterval(updateHeartbeatWidget, 15000);
  setInterval(() => { if (S.section === 'overview') loadOverview(); }, 10000);
  setInterval(() => { if (S.section === 'approvals') loadApprovals(); }, 15000);
})();
</script>
</body>
</html>"#;
