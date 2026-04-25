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
    types::{AgentsAddBody, BoundaryAddBody, BoundaryConfirmBody, VaultSetBody},
};

pub(super) async fn api_agents_view(State(state): State<DashboardState>) -> impl IntoResponse {
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let content = std::fs::read_to_string(&paths.agents_file).unwrap_or_default();
    Json(json!({ "content": content })).into_response()
}

pub(super) async fn api_agents_add(
    State(state): State<DashboardState>,
    Json(body): Json<AgentsAddBody>,
) -> impl IntoResponse {
    if body.note.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "note is required").into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let mut content = std::fs::read_to_string(&paths.agents_file).unwrap_or_default();
    let section_header = format!("## {}", body.section);
    if content.contains(&section_header) {
        if let Some(pos) = content.find(&section_header) {
            let end = content[pos..].find("\n## ").map(|i| pos + i).unwrap_or(content.len());
            content.insert_str(end, &format!("\n- {}", body.note.trim()));
        }
    } else {
        content.push_str(&format!("\n\n{section_header}\n\n- {}\n", body.note.trim()));
    }
    match std::fs::write(&paths.agents_file, &content) {
        Ok(()) => Json(json!({ "added": true })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_vault_list(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::vault::{Vault, VaultEntry};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let vault = Vault::load(&paths.vault_file).unwrap_or_default();
    let entries: Vec<_> = vault
        .secrets
        .iter()
        .map(|(name, entry)| match entry {
            VaultEntry::Literal { .. } => json!({ "name": name, "kind": "literal" }),
            VaultEntry::EnvVar { env, fallback } => json!({
                "name": name,
                "kind": "env",
                "env": env,
                "has_fallback": fallback.is_some(),
            }),
        })
        .collect();
    Json(json!({ "entries": entries })).into_response()
}

pub(super) async fn api_vault_set(
    State(state): State<DashboardState>,
    Json(body): Json<VaultSetBody>,
) -> impl IntoResponse {
    use crate::vault::{Vault, VaultEntry};
    if body.name.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let mut vault = Vault::load(&paths.vault_file).unwrap_or_default();
    let entry = match body.kind.as_str() {
        "literal" => {
            let Some(value) = body.value else {
                return (StatusCode::BAD_REQUEST, "value required for literal kind")
                    .into_response();
            };
            VaultEntry::Literal { value }
        }
        "env" => {
            let Some(env) = body.env else {
                return (StatusCode::BAD_REQUEST, "env required for env kind").into_response();
            };
            VaultEntry::EnvVar { env, fallback: body.fallback }
        }
        _ => return (StatusCode::BAD_REQUEST, "kind must be 'literal' or 'env'").into_response(),
    };
    vault.secrets.insert(body.name.trim().to_string(), entry);
    match vault.save(&paths.vault_file) {
        Ok(()) => Json(json!({ "saved": true, "name": body.name })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_vault_delete(
    State(state): State<DashboardState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    use crate::vault::Vault;
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let mut vault = Vault::load(&paths.vault_file).unwrap_or_default();
    if vault.secrets.remove(&name).is_none() {
        return (StatusCode::NOT_FOUND, "entry not found").into_response();
    }
    match vault.save(&paths.vault_file) {
        Ok(()) => Json(json!({ "deleted": true, "name": name })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_boundaries_list(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::{
        boundaries::{BoundaryReviewState, list_boundaries, review_prompt},
        time::{Clock, SystemClock},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    let state_val =
        BoundaryReviewState::load_or_default(&paths.boundary_review_file).unwrap_or_default();
    let rules = list_boundaries(&paths.identity_file).unwrap_or_default();
    Json(json!({
        "rules": rules,
        "review_due": state_val.review_due(now),
        "last_confirmed_at": state_val.last_confirmed_at,
        "last_note": state_val.last_note,
        "review_prompt": review_prompt(&state_val, now),
    }))
    .into_response()
}

pub(super) async fn api_boundaries_add(
    State(state): State<DashboardState>,
    Json(body): Json<BoundaryAddBody>,
) -> impl IntoResponse {
    use crate::boundaries::add_boundary;
    if body.rule.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "rule is required").into_response();
    }
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match add_boundary(&paths.identity_file, body.rule.trim()) {
        Ok(()) => Json(json!({ "added": true, "rule": body.rule })).into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_boundaries_confirm(
    State(state): State<DashboardState>,
    Json(body): Json<BoundaryConfirmBody>,
) -> impl IntoResponse {
    use crate::{
        boundaries::confirm_review,
        time::{Clock, SystemClock},
    };
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let now = match SystemClock::from_env().map(|c| c.now_utc()) {
        Ok(t) => t,
        Err(e) => return api_error(e).into_response(),
    };
    match confirm_review(&paths.boundary_review_file, now, body.note.as_deref()) {
        Ok(s) => Json(json!({
            "confirmed": true,
            "last_confirmed_at": s.last_confirmed_at,
            "last_note": s.last_note,
        }))
        .into_response(),
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_forensics(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::forensics::{latest_started_at, load_snapshots};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    let started_at = match latest_started_at(&paths.database_file) {
        Ok(Some(s)) => s,
        Ok(None) => return Json(json!({ "snapshots": [] })).into_response(),
        Err(e) => return api_error(e).into_response(),
    };
    match load_snapshots(&paths.database_file, &started_at) {
        Ok(snapshots) => {
            let rows: Vec<_> = snapshots
                .iter()
                .map(|s| {
                    json!({
                        "recorded_at": s.recorded_at,
                        "checkpoint": s.checkpoint,
                        "phase": s.phase,
                        "outcome": s.state.last_outcome,
                        "session_id": s.session_id,
                    })
                })
                .collect();
            Json(json!({ "started_at": started_at, "snapshots": rows })).into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}

pub(super) async fn api_argus(State(state): State<DashboardState>) -> impl IntoResponse {
    use crate::argus::{analyze, render};
    let paths = PraxisPaths::for_data_dir(state.data_dir.clone());
    match analyze(&paths.database_file, 20) {
        Ok(report) => {
            let text = render(&report);
            Json(json!({
                "review_failures": report.review_failures,
                "eval_failures": report.eval_failures,
                "drift_status": report.drift.status.as_str(),
                "drift_recent_score": report.drift.recent_score,
                "drift_baseline_score": report.drift.baseline_score,
                "repeated_work": report.repeated_work.iter().map(|p| json!({
                    "label": p.label,
                    "sessions": p.sessions,
                    "distinct_days": p.distinct_days,
                    "latest_outcome": p.latest_outcome,
                })).collect::<Vec<_>>(),
                "token_hotspots": report.token_hotspots.iter().map(|(provider, model, tokens)| json!({
                    "provider": provider,
                    "model": model,
                    "tokens": tokens,
                })).collect::<Vec<_>>(),
                "report_text": text,
            }))
            .into_response()
        }
        Err(e) => api_error(e).into_response(),
    }
}
