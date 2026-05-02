use std::{convert::Infallible, time::Duration};

use async_stream::stream;
use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
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
        .map(|(items, _)| items.into_iter().rev().take(20).collect::<Vec<PraxisEvent>>())
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

    let hot_count = store.recent_hot_memories(10_000).map(|v| v.len()).unwrap_or(0);
    gauge!("praxis_hot_memory_count", "Number of hot memories.", hot_count);

    let cold_count = store.strongest_cold_memories(10_000).map(|v| v.len()).unwrap_or(0);
    gauge!("praxis_cold_memory_count", "Number of cold memories.", cold_count);

    let pending = store
        .list_approvals(Some(ApprovalStatus::Pending))
        .map(|v| v.len())
        .unwrap_or(0);
    gauge!("praxis_approvals_pending", "Pending tool-approval requests.", pending);

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

    let session_count = store.last_session().ok().flatten().map(|s| s.id).unwrap_or(0);
    gauge!("praxis_sessions_total", "Total sessions (by last session ID).", session_count);

    // Token usage metrics
    if let Ok(summary) = store.token_summary_all_time() {
        gauge!(
            "praxis_tokens_total",
            "Total tokens consumed across all sessions.",
            summary.total_tokens
        );
        gauge!(
            "praxis_cost_micros_total",
            "Total estimated cost in micro-dollars.",
            summary.total_cost_micros
        );
    }

    if let Ok(by_provider) = store.token_usage_by_provider() {
        for p in by_provider {
            lines.push(format!(
                "# HELP praxis_provider_tokens_total Tokens by provider.\n# TYPE praxis_provider_tokens_total gauge\npraxis_provider_tokens_total{{provider=\"{}\"}} {}",
                p.provider, p.tokens_used
            ));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

#[cfg(feature = "discord")]
pub(super) async fn webhook_discord(
    State(state): State<DashboardState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Fail closed: require the public key to be configured.
    let public_key_hex = match state.discord_public_key.as_deref() {
        Some(key) => key,
        None => {
            log::error!("discord webhook rejected: PRAXIS_DISCORD_PUBLIC_KEY not configured");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let signature_hex = match headers.get("X-Signature-Ed25519").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let timestamp = match headers.get("X-Signature-Timestamp").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // Verify ED25519 signature: sign(timestamp || body) with app public key.
    let verifying_key = match hex::decode(public_key_hex)
        .ok()
        .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
        .map(|bytes| VerifyingKey::from_bytes(&bytes))
    {
        Some(Ok(pk)) => pk,
        _ => {
            log::error!("discord webhook rejected: invalid public key");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let sig_bytes = match hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let signature = match Signature::try_from(sig_bytes.as_slice()) {
        Ok(s) => s,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let mut message = Vec::with_capacity(timestamp.len() + body.len());
    message.extend_from_slice(timestamp.as_bytes());
    message.extend_from_slice(&body);

    if verifying_key.verify(&message, &signature).is_err() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let body: crate::messaging::discord::DiscordInteraction = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("discord webhook: invalid JSON body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    use crate::wakeup::{WakeIntent, request_wake};
    if body.interaction_type == 1 {
        return (StatusCode::OK, Json(json!({ "type": 1 }))).into_response();
    }
    let task = body.data.as_ref().and_then(|d| d.name.as_deref()).map(str::to_string);
    let reason = task.clone().unwrap_or_else(|| "discord interaction".to_string());
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
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Fail closed: require the signing secret to be configured.
    let signing_secret = match state.slack_signing_secret.as_deref() {
        Some(secret) => secret,
        None => {
            log::error!("slack webhook rejected: PRAXIS_SLACK_SIGNING_SECRET not configured");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };

    let signature = match headers.get("X-Slack-Signature").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let timestamp = match headers.get("X-Slack-Request-Timestamp").and_then(|v| v.to_str().ok()) {
        Some(s) => s,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // Reject requests older than 5 minutes to prevent replay attacks.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ts = timestamp.parse::<u64>().unwrap_or(0);
    if ts + 300 < now {
        log::warn!("slack webhook rejected: timestamp too old");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Verify HMAC-SHA256 signature: v0=<hex(hmac_sha256("v0:timestamp:body", secret))>.
    let signature_hex = match signature.strip_prefix("v0=") {
        Some(h) => h,
        None => {
            log::warn!("slack webhook rejected: invalid signature format");
            return StatusCode::UNAUTHORIZED.into_response();
        }
    };
    let signature_bytes = match hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let basestring = format!("v0:{timestamp}:");
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = match HmacSha256::new_from_slice(signing_secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    mac.update(basestring.as_bytes());
    mac.update(&body);
    if mac.verify_slice(&signature_bytes).is_err() {
        log::warn!("slack webhook rejected: signature mismatch");
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let body: crate::messaging::slack::SlackEvent = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("slack webhook: invalid JSON body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

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

/// Dynamic webhook handler — routes `/webhook/{name}` to registered subscriptions.
/// If `direct_delivery` is set on the webhook, the payload is forwarded to the
/// messaging bus. Otherwise, a `WakeIntent` is created for the agent loop.
pub(super) async fn webhook_dynamic(
    State(state): State<DashboardState>,
    axum::extract::Path(name): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    use crate::bus::{BusEvent, FileBus, MessageBus};
    use crate::wakeup::{WakeIntent, request_wake};
    use crate::webhooks::WebhookStore;

    let mut store = match WebhookStore::load(&state.data_dir.join("webhooks.json")) {
        Ok(s) => s,
        Err(e) => {
            log::error!("webhook: failed to load store: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let wh = match store.get(&name) {
        Some(w) => w.clone(),
        None => {
            log::warn!("webhook: unknown subscription '{name}'");
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    // Verify HMAC signature if secret is set.
    if wh.secret.is_some() {
        let timestamp =
            headers.get("X-Webhook-Timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
        let signature = headers.get("X-Signature-256").and_then(|v| v.to_str().ok()).unwrap_or("");

        if timestamp.is_empty() || signature.is_empty() {
            log::warn!("webhook '{name}': missing signature headers");
            return StatusCode::UNAUTHORIZED.into_response();
        }

        match wh.verify_signature(timestamp, &body, signature) {
            Ok(true) => {}
            Ok(false) => {
                log::warn!("webhook '{name}': signature verification failed");
                return StatusCode::UNAUTHORIZED.into_response();
            }
            Err(e) => {
                log::warn!("webhook '{name}': signature error: {e}");
                return StatusCode::BAD_REQUEST.into_response();
            }
        }
    }

    // Check event type filter.
    if !wh.events.is_empty() {
        let event_type =
            headers.get("X-Webhook-Event").and_then(|v| v.to_str().ok()).unwrap_or("push");
        let allowed: Vec<&str> = wh.events.split(',').map(|s| s.trim()).collect();
        if !allowed.iter().any(|e| *e == event_type) && !allowed.iter().any(|e| *e == "*") {
            log::info!("webhook '{name}': event '{event_type}' not in allowed list");
            return (StatusCode::OK, Json(json!({ "ok": true, "skipped": true }))).into_response();
        }
    }

    // Update trigger stats.
    if let Some(wh) = store.webhooks.iter_mut().find(|w| w.name == name) {
        wh.last_triggered_at = Some(chrono::Utc::now());
        wh.trigger_count += 1;
    }
    let _ = store.save(&state.data_dir.join("webhooks.json"));

    let payload = String::from_utf8_lossy(&body).to_string();

    if wh.direct_delivery {
        // Forward directly to the messaging bus — no agent processing.
        let bus = FileBus::new(state.data_dir.join("bus.jsonl"));
        let event =
            BusEvent::new("webhook", "webhook", &format!("webhook:{name}"), "system", &payload);
        if let Err(e) = bus.publish(&event) {
            log::warn!("webhook '{name}': direct delivery publish failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        log::info!("webhook '{name}': direct delivery to bus");
    } else {
        // Create a WakeIntent for the agent loop.
        let reason = format!("webhook:{name}: {payload}");
        let intent = WakeIntent::new(&reason, "webhook").with_task(payload);
        if let Err(e) = request_wake(&state.data_dir, &intent) {
            log::warn!("webhook '{name}': wake intent failed: {e}");
        }
        log::info!("webhook '{name}': wake intent created");
    }

    (StatusCode::OK, Json(json!({ "ok": true }))).into_response()
}
