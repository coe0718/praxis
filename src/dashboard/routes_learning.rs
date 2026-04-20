use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::paths::PraxisPaths;

use super::{
    server::{DashboardState, api_error},
    types::LearningNoteBody,
};

pub(super) async fn api_learning_list(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{
        learning::OpportunityStatus,
        storage::{SessionStore, SqliteSessionStore},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    if let Err(e) = store.initialize() {
        return api_error(e).into_response();
    }
    let pending = store
        .list_opportunities(OpportunityStatus::Pending, 50)
        .unwrap_or_default();
    let accepted = store
        .list_opportunities(OpportunityStatus::Accepted, 50)
        .unwrap_or_default();
    let dismissed = store
        .list_opportunities(OpportunityStatus::Dismissed, 50)
        .unwrap_or_default();
    let latest_run = store.latest_learning_run().unwrap_or(None);
    Json(json!({
        "latest_run": latest_run,
        "pending": pending,
        "accepted": accepted,
        "dismissed": dismissed,
    }))
    .into_response()
}

pub(super) async fn api_learning_note(
    State(state): State<DashboardState>,
    Json(body): Json<LearningNoteBody>,
) -> impl IntoResponse {
    use crate::{
        learning::append_note,
        time::{Clock, SystemClock},
    };
    if body.text.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "text is required").into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    match append_note(&paths, &body.text, now) {
        Ok(entry) => Json(json!({
            "added": true,
            "summary": entry.summary,
            "appended_at": entry.appended_at,
        }))
        .into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_learning_run(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{
        learning::run_once,
        storage::{SessionStore, SqliteSessionStore},
        time::{Clock, SystemClock},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    if let Err(e) = store.initialize() {
        return api_error(e).into_response();
    }
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    match run_once(&paths, &store, now) {
        Ok(result) => Json(json!({
            "processed_sources": result.processed_sources,
            "changed_sources": result.changed_sources,
            "opportunities_created": result.opportunities_created,
            "throttle_reached": result.throttle_reached,
            "notes": result.notes,
        }))
        .into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_learning_accept(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::{
        learning::{OpportunityStatus, update_opportunity},
        storage::{SessionStore, SqliteSessionStore},
        time::{Clock, SystemClock},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    if let Err(e) = store.initialize() {
        return api_error(e).into_response();
    }
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    match update_opportunity(&paths, &store, id, OpportunityStatus::Accepted, now) {
        Ok(Some(op)) => Json(json!({
            "id": id,
            "status": "accepted",
            "opportunity": op.opportunity,
            "promoted_goal_id": op.promoted_goal_id,
            "created_goal": op.created_goal,
        }))
        .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "opportunity not found").into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_learning_dismiss(
    State(state): State<DashboardState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    use crate::{
        learning::{OpportunityStatus, update_opportunity},
        storage::{SessionStore, SqliteSessionStore},
        time::{Clock, SystemClock},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = SqliteSessionStore::new(paths.database_file.clone());
    if let Err(e) = store.initialize() {
        return api_error(e).into_response();
    }
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    match update_opportunity(&paths, &store, id, OpportunityStatus::Dismissed, now) {
        Ok(Some(op)) => Json(json!({
            "id": id,
            "status": "dismissed",
            "opportunity": op.opportunity,
        }))
        .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "opportunity not found").into_response(),
        Err(e) => api_error(e).into_response(),
    }
}
