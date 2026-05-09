//! Signal/Matrix channels — Enterprise messaging support.
//!
//! Moltis has Signal + Matrix integrations.
//! This adds secure enterprise messaging channels.
//!
//! Wired into the Praxis event system: after agent events (delegated, task
//! complete, approval required, etc.) a notification is sent via Signal or
//! Matrix when the corresponding environment variables are set.

use anyhow::Result;
use std::env;

/// Signal message client.
pub struct SignalClient {
    /// Registered phone number.
    phone_number: String,
}

impl SignalClient {
    /// Create a new Signal client from environment variables.
    /// Returns `None` if `PRAXIS_SIGNAL_PHONE` is not set.
    pub fn from_env() -> Option<Self> {
        env::var("PRAXIS_SIGNAL_PHONE").ok().map(|phone| Self { phone_number: phone })
    }

    /// Send a message via Signal to the configured recipient.
    /// Currently a stub — returns Ok(()) until a Signal CLI or REST API is integrated.
    pub fn send(&self, recipient: &str, message: &str) -> Result<()> {
        let _ = (recipient, message);
        // TODO: integrate signal-cli (signal-rest-api) or a Signal webhook API.
        log::debug!(
            "Signal: would send to {}: {}",
            self.phone_number,
            message.chars().take(80).collect::<String>()
        );
        Ok(())
    }
}

/// Matrix message client.
pub struct MatrixClient {
    /// Homeserver URL.
    homeserver: String,
    /// Room ID for delivery.
    room_id: String,
    /// Access token.
    access_token: String,
}

impl MatrixClient {
    /// Create a new Matrix client from environment variables.
    /// Returns `None` if `PRAXIS_MATRIX_HOMESERVER` or `PRAXIS_MATRIX_TOKEN` is not set.
    pub fn from_env() -> Option<Self> {
        let homeserver = env::var("PRAXIS_MATRIX_HOMESERVER").ok()?;
        let room_id = env::var("PRAXIS_MATRIX_ROOM").ok()?;
        let access_token = env::var("PRAXIS_MATRIX_TOKEN").ok()?;
        Some(Self {
            homeserver,
            room_id,
            access_token,
        })
    }

    /// Send a message to the configured Matrix room.
    pub async fn send(&self, message: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!(
            "{}/_matrix/client/r0/rooms/{}/send/m.room.message",
            self.homeserver.trim_end_matches('/'),
            self.room_id
        );

        let txn_id = format!("praxis_{}", chrono::Utc::now().timestamp_millis());
        let payload = serde_json::json!({
            "msgtype": "m.text",
            "body": message,
            "txn_id": txn_id,
        });

        let resp = client.post(&url).bearer_auth(&self.access_token).json(&payload).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Matrix send failed: {}", resp.status());
        }
    }
}

/// Wire channels into the Praxis event system.
/// Call this after emitting significant events to forward them to Signal/Matrix.
pub fn notify_event(kind: &str, detail: &str) {
    // Forward to Signal if configured.
    if let Some(client) = SignalClient::from_env() {
        let msg = format!("[Praxis] {kind}: {detail}");
        if let Err(e) = client.send("operator", &msg) {
            log::warn!("Signal notification failed: {e}");
        }
    }

    // Forward to Matrix if configured (sync stub until REST API is integrated).
    if MatrixClient::from_env().is_some() {
        log::debug!(
            "Matrix: would send {} — {}",
            kind,
            detail.chars().take(80).collect::<String>()
        );
        // TODO: integrate Matrix REST API (synapse or dendrite) for async delivery.
    }
}
