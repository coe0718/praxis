//! Slack Events API receiver for push-based message delivery.
//!
//! Instead of polling the Slack REST API, this module handles the
//! Slack Events API challenge handshake and event subscriptions,
//! publishing inbound events to the Praxis BusEvent system.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Slack Events API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackEventsConfig {
    /// The signing secret for request verification.
    pub signing_secret: String,
    /// The Bot User OAuth token (xoxb-...).
    pub bot_token: String,
}

/// Slack URL verification challenge request.
#[derive(Debug, Deserialize)]
pub struct UrlVerificationRequest {
    pub challenge: String,
    pub token: String,
}

/// Slack URL verification challenge response.
#[derive(Debug, Serialize)]
pub struct UrlVerificationResponse {
    pub challenge: String,
}

/// Handle the Slack URL verification challenge.
///
/// When Slack first registers an Events API endpoint, it sends
/// a challenge request. The handler must echo back the `challenge`
/// value to prove ownership.
pub fn handle_url_challenge(body: &str) -> Result<String> {
    let request: UrlVerificationRequest = serde_json::from_str(body)
        .map_err(|e| anyhow::anyhow!("invalid challenge request: {e}"))?;

    let response = UrlVerificationResponse { challenge: request.challenge };

    serde_json::to_string(&response)
        .map_err(|e| anyhow::anyhow!("failed to serialize challenge response: {e}"))
}

/// Verify Slack request signature using HMAC-SHA256.
///
/// Slack signs every Events API request with the signing secret.
/// This function verifies the signature to prevent spoofing.
pub fn verify_slack_signature(
    signing_secret: &str,
    timestamp: &str,
    body: &str,
    signature: &str,
) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let sig_base = format!("{timestamp}:{body}");
    let Ok(mut mac) = HmacSha256::new_from_slice(signing_secret.as_bytes()) else {
        return false;
    };
    mac.update(sig_base.as_bytes());
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    let computed = format!("v0={}", hex::encode(code_bytes));

    computed == signature
}

/// Extract the event type from a Slack Events API payload.
#[derive(Debug, Deserialize)]
pub struct SlackEventPayload {
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub event: Option<serde_json::Value>,
}

/// Parse a Slack Events API payload and return the inner event.
pub fn parse_event_payload(body: &str) -> Result<SlackEventPayload> {
    serde_json::from_str(body).map_err(|e| anyhow::anyhow!("invalid Slack event payload: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_challenge() {
        let body = r#"{"challenge":"test_challenge_123","token":"verification_token"}"#;
        let response = handle_url_challenge(body).unwrap();
        assert!(response.contains("test_challenge_123"));
    }
}
