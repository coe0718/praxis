use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;

use crate::{
    paths::PraxisPaths,
    wakeup::{WakeIntent, request_wake},
};

use super::{
    server::{DashboardState, api_error},
    types::{AskBody, RunBody, WakeBody},
};

pub(super) async fn api_wake(
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_run(
    State(state): State<DashboardState>,
    Json(body): Json<RunBody>,
) -> impl IntoResponse {
    use crate::cli::{RunArgs, core::handle_run};
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_ask(
    State(state): State<DashboardState>,
    Json(body): Json<AskBody>,
) -> impl IntoResponse {
    use crate::cli::{AskArgs, core::handle_ask};
    if body.prompt.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "prompt is required").into_response();
    }
    match handle_ask(
        Some(state.data_dir.clone()),
        AskArgs {
            files: vec![],
            attachment_policy: "reject".to_string(),
            tools: false,
            prompt: vec![body.prompt],
        },
    ) {
        Ok(output) => Json(json!({ "output": output })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_canary_run(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::cli::canary::{CanaryArgs, CanaryCommand, CanaryRunArgs, handle_canary};
    match handle_canary(
        Some(state.data_dir.clone()),
        CanaryArgs {
            command: CanaryCommand::Run(CanaryRunArgs { provider: None }),
        },
    ) {
        Ok(output) => Json(json!({ "output": output })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_evolution(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let store = crate::evolution::EvolutionStore::from_paths(&paths);
    match store.all() {
        Ok(proposals) => Json(proposals).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_evolution_approve(
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
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_delegation(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::delegation::DelegationStore;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match DelegationStore::load(&paths.delegation_links_file) {
        Ok(store) => Json(json!({
            "links": store.links.values().collect::<Vec<_>>(),
            "active_counts": store.active_counts,
        }))
        .into_response(),
        Err(e) => api_error(e).into_response(),
    }
}
