use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;

use crate::paths::PraxisPaths;

use super::{
    server::{DashboardState, api_error},
    types::{ApprovalRow, MemoryRow},
};

pub(super) async fn api_memories_hot(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_memories_cold(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_memories_consolidate(
    State(state): State<DashboardState>,
) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
    use chrono::Utc;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.consolidate_memories(Utc::now()) {
        Ok(s) => {
            Json(json!({ "consolidated": s.consolidated, "pruned": s.pruned })).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_memory_reinforce(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.boost_memory(id) {
        Ok(_) => Json(json!({ "id": id, "action": "reinforced" })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_memory_forget(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.forget_memory(id) {
        Ok(_) => Json(json!({ "id": id, "action": "forgotten" })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct MemorySearchQuery {
    q: String,
}

pub(super) async fn api_memories_search(
    State(state): State<DashboardState>,
    Query(query): Query<MemorySearchQuery>,
) -> impl IntoResponse {
    use crate::{memory::MemoryStore, storage::SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    if query.q.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "q is required").into_response();
    }
    match store.search_memories(&query.q, 50) {
        Ok(mems) => {
            let rows: Vec<MemoryRow> = mems
                .into_iter()
                .map(|m| MemoryRow {
                    id: m.id,
                    tier: match m.tier {
                        crate::memory::MemoryTier::Hot => "hot".to_string(),
                        crate::memory::MemoryTier::Cold => "cold".to_string(),
                    },
                    content: m.content,
                    summary: m.summary,
                    tags: m.tags,
                    score: m.score as f64,
                    memory_type: m.memory_type.as_str().to_string(),
                })
                .collect();
            Json(rows).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct ApprovalQuery {
    q: Option<String>,
    tool: Option<String>,
    status: Option<String>,
}

pub(super) async fn api_approvals(
    State(state): State<DashboardState>,
    Query(query): Query<ApprovalQuery>,
) -> impl IntoResponse {
    use crate::storage::{ApprovalStatus, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());

    let status = query.status.as_deref().and_then(|s| ApprovalStatus::parse(s).ok());

    match store.search_approvals(query.q.as_deref(), query.tool.as_deref(), status) {
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_approve(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.set_approval_status(id, ApprovalStatus::Approved, None) {
        Ok(Some(r)) => Json(json!({ "id": r.id, "status": r.status.as_str() })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "approval not found").into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_reject(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    match store.set_approval_status(id, ApprovalStatus::Rejected, None) {
        Ok(Some(r)) => Json(json!({ "id": r.id, "status": r.status.as_str() })).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "approval not found").into_response(),
        Err(e) => api_error(e).into_response(),
    }
}
