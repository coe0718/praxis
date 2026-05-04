//! Dashboard plugin tab routes — third-party tabs and widgets.  (#19)
//!
//! Plugins can register dashboard tabs via their `plugin.toml`.  Each tab
//! has a name, an icon, and an HTML snippet URL that the SPA fetches and
//! renders inside a tab panel.  This module exposes the REST API the SPA
//! uses to discover available plugin tabs.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Serialize;
use serde_json::json;

use crate::dashboard::server::DashboardState;

// ── Types ──────────────────────────────────────────────────────

/// A single dashboard tab registered by a plugin.
#[derive(Debug, Serialize)]
pub struct PluginTab {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    /// URL that returns the HTML content for this tab's panel.
    pub content_url: String,
    /// Position hint (lower = first).  Default 100.
    pub order: u32,
}

// ── Routes ─────────────────────────────────────────────────────

/// `GET /api/plugins/tabs` — list all dashboard tabs from loaded plugins.
///
/// Returns a JSON array of `PluginTab` objects.  The SPA uses this to
/// dynamically render third-party tabs alongside the built-in ones.
pub async fn list_plugin_tabs(State(_state): State<DashboardState>) -> Response {
    // In the current implementation, plugin tabs are discovered from the
    // plugins directory at startup.  For now, return an empty array —
    // the infrastructure is in place for plugins to register tabs via
    // `[dashboard]` sections in their `plugin.toml`.
    //
    // Example plugin.toml:
    //   [dashboard]
    //   [[dashboard.tabs]]
    //   id = "analytics"
    //   label = "Analytics"
    //   icon = "chart"
    //   content_url = "/plugins/analytics/widget.html"
    //   order = 50
    let tabs: Vec<PluginTab> = vec![];
    (StatusCode::OK, Json(json!({ "tabs": tabs }))).into_response()
}

/// `GET /api/plugins/widgets/:name` — placeholder for plugin widget content.
///
/// In a full implementation, this would serve the HTML content from the
/// plugin's directory.  For now, returns 404 for any unknown widget.
pub async fn get_plugin_widget(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "plugin widget not found",
            "name": name
        })),
    )
        .into_response()
}
