use std::path::PathBuf;

use anyhow::Result;
use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
    routing::{delete, get, post},
};

use super::{
    routes_admin, routes_control, routes_core, routes_events, routes_learning, routes_memory,
};

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub(super) struct DashboardState {
    pub data_dir: PathBuf,
    /// Bearer token required on all API requests.  `None` when
    /// `PRAXIS_DASHBOARD_TOKEN` is unset — all requests are allowed but a
    /// warning is logged at startup.
    pub token: Option<String>,
}

// ── Auth middleware ───────────────────────────────────────────────────────────

pub(super) async fn require_auth(
    State(state): State<DashboardState>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(ref expected) = state.token {
        let auth_ok = request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|t| t == expected)
            .unwrap_or(false);
        if !auth_ok {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
    next.run(request).await
}

use axum::response::IntoResponse;

pub(super) fn api_error(e: impl std::fmt::Display) -> (StatusCode, &'static str) {
    log::error!("dashboard: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

pub(super) async fn add_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; connect-src 'self'",
        ),
    );
    response
}

// ── Router ────────────────────────────────────────────────────────────────────

pub async fn serve_dashboard(data_dir: PathBuf, host: String, port: u16) -> Result<()> {
    let token = std::env::var("PRAXIS_DASHBOARD_TOKEN").ok();
    if token.is_none() {
        log::warn!("dashboard: PRAXIS_DASHBOARD_TOKEN not set — all endpoints are unauthenticated");
    }
    let state = DashboardState { data_dir, token };

    // SSE stream is read-only and exempt from auth — EventSource API cannot send headers.
    let public_routes = Router::new()
        .route("/events", get(routes_events::events_sse))
        .with_state(state.clone());

    let app = Router::new()
        .route("/", get(routes_events::index))
        .route("/status", get(routes_events::status_text))
        .route("/health", get(routes_events::health))
        .route("/metrics", get(routes_events::prometheus_metrics))
        .route("/events/recent", get(routes_events::recent_events))
        .route("/mcp", post(routes_events::mcp_endpoint))
        // Core
        .route("/api/summary", get(routes_core::api_summary))
        .route("/api/sessions", get(routes_core::api_sessions))
        .route(
            "/api/goals",
            get(routes_core::api_goals).post(routes_core::api_goals_add),
        )
        .route(
            "/api/identity/:file",
            get(routes_core::api_identity_read).put(routes_core::api_identity_write),
        )
        .route("/api/config", get(routes_core::api_config))
        .route("/api/tools", get(routes_core::api_tools))
        .route("/api/heartbeat", get(routes_core::api_heartbeat))
        .route("/api/score", get(routes_core::api_score))
        .route("/api/report", get(routes_core::api_report))
        .route("/api/canary", get(routes_core::api_canary))
        // Memory & approvals
        .route("/api/memories/hot", get(routes_memory::api_memories_hot))
        .route("/api/memories/cold", get(routes_memory::api_memories_cold))
        .route(
            "/api/memories/consolidate",
            post(routes_memory::api_memories_consolidate),
        )
        .route(
            "/api/memories/:id/reinforce",
            post(routes_memory::api_memory_reinforce),
        )
        .route(
            "/api/memories/:id/forget",
            post(routes_memory::api_memory_forget),
        )
        .route("/api/approvals", get(routes_memory::api_approvals))
        .route(
            "/api/approvals/:id/approve",
            post(routes_memory::api_approve),
        )
        .route("/api/approvals/:id/reject", post(routes_memory::api_reject))
        // Control
        .route("/api/wake", post(routes_control::api_wake))
        .route("/api/run", post(routes_control::api_run))
        .route("/api/ask", post(routes_control::api_ask))
        .route("/api/canary/run", post(routes_control::api_canary_run))
        .route("/api/evolution", get(routes_control::api_evolution))
        .route(
            "/api/evolution/:id/approve",
            post(routes_control::api_evolution_approve),
        )
        .route("/api/delegation", get(routes_control::api_delegation))
        // Learning
        .route("/api/learning", get(routes_learning::api_learning_list))
        .route(
            "/api/learning/note",
            post(routes_learning::api_learning_note),
        )
        .route("/api/learning/run", post(routes_learning::api_learning_run))
        .route(
            "/api/learning/:id/accept",
            post(routes_learning::api_learning_accept),
        )
        .route(
            "/api/learning/:id/dismiss",
            post(routes_learning::api_learning_dismiss),
        )
        // Admin
        .route(
            "/api/agents",
            get(routes_admin::api_agents_view).post(routes_admin::api_agents_add),
        )
        .route(
            "/api/vault",
            get(routes_admin::api_vault_list).post(routes_admin::api_vault_set),
        )
        .route("/api/vault/:name", delete(routes_admin::api_vault_delete))
        .route(
            "/api/boundaries",
            get(routes_admin::api_boundaries_list).post(routes_admin::api_boundaries_add),
        )
        .route(
            "/api/boundaries/confirm",
            post(routes_admin::api_boundaries_confirm),
        )
        .route("/api/forensics", get(routes_admin::api_forensics))
        .route("/api/argus", get(routes_admin::api_argus));

    #[cfg(feature = "discord")]
    let app = app.route("/webhook/discord", post(routes_events::webhook_discord));
    #[cfg(feature = "slack")]
    let app = app.route("/webhook/slack", post(routes_events::webhook_slack));

    let app = app
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state);

    let app = Router::new()
        .merge(public_routes)
        .merge(app)
        .layer(axum::middleware::from_fn(add_security_headers));

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    log::info!("dashboard: listening on http://{host}:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
