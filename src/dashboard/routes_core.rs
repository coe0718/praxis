use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::paths::PraxisPaths;

use super::{
    helpers::{
        append_goal, parse_goals_file, query_recent_sessions, read_jsonl_tail,
        resolve_identity_file,
    },
    server::{DashboardState, api_error},
    types::{AddGoalBody, WriteFileBody},
};

pub(super) async fn api_summary(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{cli::core, report::build_status_report};
    match core::load_initialized_config(Some(state.data_dir.clone()))
        .and_then(|(config, paths)| build_status_report(&config, &paths))
    {
        Ok(report) => Json(serde_json::to_value(report).unwrap_or(json!({}))).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_sessions(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match query_recent_sessions(&paths.database_file, 50) {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_goals(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match parse_goals_file(&paths.goals_file) {
        Ok(goals) => Json(goals).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_goals_add(
    State(state): State<DashboardState>,
    Json(body): Json<AddGoalBody>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    if body.description.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "description is required").into_response();
    }
    match append_goal(&paths.goals_file, &body.description) {
        Ok(id) => Json(json!({ "id": id, "title": body.description })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_identity_read(
    State(state): State<DashboardState>,
    Path(file): Path<String>,
) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let path = match resolve_identity_file(&paths, &file) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "unknown identity file").into_response(),
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            Json(json!({ "file": file, "content": content, "writable": file != "soul" }))
                .into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Json(json!({ "file": file, "content": "", "writable": file != "soul" })).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

const MAX_IDENTITY_FILE_SIZE: usize = 512 * 1024; // 512 KB

pub(super) async fn api_identity_write(
    State(state): State<DashboardState>,
    Path(file): Path<String>,
    Json(body): Json<WriteFileBody>,
) -> impl IntoResponse {
    if file == "soul" {
        return (StatusCode::FORBIDDEN, "SOUL.md is immutable").into_response();
    }
    if body.content.len() > MAX_IDENTITY_FILE_SIZE {
        return (StatusCode::PAYLOAD_TOO_LARGE, "content exceeds maximum file size")
            .into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let path = match resolve_identity_file(&paths, &file) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "unknown identity file").into_response(),
    };
    match std::fs::write(&path, &body.content) {
        Ok(()) => Json(json!({ "file": file, "saved": true })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_config(State(state): State<DashboardState>) -> impl IntoResponse {
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

pub(super) async fn api_tools(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::tools::{FileToolRegistry, ToolRegistry};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match FileToolRegistry.list(&paths) {
        Ok(tools) => Json(tools).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_heartbeat(State(state): State<DashboardState>) -> impl IntoResponse {
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

pub(super) async fn api_score(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match read_jsonl_tail(&paths.score_file, 30) {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_report(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::cli::core::handle_status;
    match handle_status(Some(state.data_dir.clone())) {
        Ok(text) => Json(json!({ "report": text })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_canary(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::canary::{CanaryFreezeState, ModelCanaryLedger};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let ledger = ModelCanaryLedger::load_or_default(&paths.model_canary_file)
        .unwrap_or_else(|_| ModelCanaryLedger { records: vec![] });
    let freeze = CanaryFreezeState::load_or_default(&paths.canary_freeze_file).unwrap_or_default();
    Json(json!({ "records": ledger.records, "frozen": freeze.frozen }))
}

pub(super) async fn api_tokens(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());

    let summary = store.token_summary_all_time().unwrap_or_else(|e| {
        log::warn!("token summary query failed: {e:#}");
        crate::usage::TokenSummaryAllTime {
            total_tokens: 0,
            total_cost_micros: 0,
            total_sessions: 0,
        }
    });
    let by_provider = store.token_usage_by_provider().unwrap_or_else(|e| {
        log::warn!("token usage by provider query failed: {e:#}");
        Vec::new()
    });

    Json(json!({
        "total_tokens": summary.total_tokens,
        "total_cost_micros": summary.total_cost_micros,
        "total_sessions": summary.total_sessions,
        "by_provider": by_provider.into_iter().map(|p| json!({
            "provider": p.provider,
            "tokens_used": p.tokens_used,
            "estimated_cost_micros": p.estimated_cost_micros,
        })).collect::<Vec<_>>(),
    }))
    .into_response()
}

pub(super) async fn api_tokens_sessions(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::storage::SqliteSessionStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.token_usage_by_session(50) {
        Ok(rows) => {
            let out = rows
                .into_iter()
                .map(|r| {
                    json!({
                        "session_id": r.session_id,
                        "day": r.day,
                        "tokens_used": r.tokens_used,
                        "estimated_cost_micros": r.estimated_cost_micros,
                    })
                })
                .collect::<Vec<_>>();
            Json(out).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_health(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{
        heartbeat::read_heartbeat,
        storage::{SessionStore, SqliteSessionStore},
    };
    use chrono::{DateTime, Utc};

    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let now = Utc::now();

    let mut checks = Vec::new();
    let mut overall = "ok";

    // Config
    let config_ok = crate::cli::core::load_initialized_config(Some(state.data_dir.clone())).is_ok();
    checks.push(json!({ "name": "config", "status": if config_ok { "ok" } else { "error" } }));
    if !config_ok {
        overall = "error";
    }

    // Database
    let db_ok = store.validate_schema().is_ok();
    checks.push(json!({ "name": "database", "status": if db_ok { "ok" } else { "error" } }));
    if !db_ok {
        overall = "error";
    }

    // Heartbeat
    let hb = read_heartbeat(&paths.heartbeat_file).ok();
    let (hb_age, hb_status) = hb
        .as_ref()
        .and_then(|h| {
            DateTime::parse_from_rfc3339(&h.updated_at).ok().map(|ts| {
                let age = (now - ts.with_timezone(&Utc)).num_seconds().max(0);
                let status = if age < 300 {
                    "ok"
                } else if age < 900 {
                    "warn"
                } else {
                    "error"
                };
                (Some(age), status)
            })
        })
        .unwrap_or((None, "unknown"));

    if hb_status == "error" {
        overall = "error";
    } else if (hb_status == "warn" || hb_status == "unknown") && overall == "ok" {
        overall = "warn";
    }

    let mut hb_check = json!({
        "name": "heartbeat",
        "status": hb_status,
        "phase": hb.as_ref().map(|h| h.phase.clone()).unwrap_or_default(),
    });
    if let Some(age) = hb_age {
        hb_check["age_seconds"] = json!(age);
    }
    checks.push(hb_check);

    // Pending approvals — COUNT(*)
    let pending = store.count_pending_approvals().unwrap_or(0);
    checks.push(json!({ "name": "approvals", "status": "ok", "pending": pending }));

    // Memories — COUNT(*)
    let hot_count = store.count_hot_memories().unwrap_or(0);
    let cold_count = store.count_cold_memories().unwrap_or(0);
    checks
        .push(json!({ "name": "memories", "status": "ok", "hot": hot_count, "cold": cold_count }));

    Json(json!({
        "status": overall,
        "checked_at": now.to_rfc3339(),
        "checks": checks,
    }))
    .into_response()
}
