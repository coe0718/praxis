//! #18 Webhook Direct-Delivery
//!
//! Delivers webhook payloads directly to a chat channel, bypassing the
//! agent/LLM loop entirely.  Useful for notifications, alerts, CI/CD status
//! updates, and other payloads that don't need agent reasoning.
//!
//! When a `Webhook` has `direct_delivery = true`, the dashboard handler
//! calls `deliver_webhook_to_channel()` instead of injecting the payload
//! into the agent's wake intent.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

/// Default channel for direct delivery when no target is specified.
const DEFAULT_PLATFORM: &str = "telegram";

/// Result of a direct delivery attempt.
#[derive(Debug, Clone)]
pub struct DeliveryResult {
    /// The platform used for delivery.
    pub platform: String,
    /// The channel ID the message was sent to.
    pub channel_id: String,
    /// Whether the delivery succeeded.
    pub success: bool,
    /// Error message if delivery failed.
    pub error: Option<String>,
}

/// Route mapping for direct delivery — maps webhook names to target channels.
/// Persisted as `webhook_routes.json` in the data directory.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DeliveryRoutes {
    /// Map of webhook name → (platform, channel_id).
    #[serde(default)]
    pub routes: HashMap<String, DeliveryTarget>,
}

/// A delivery target for a webhook.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeliveryTarget {
    /// Platform: "telegram", "discord", or "slack".
    pub platform: String,
    /// Channel/chat ID to deliver to.
    pub channel_id: String,
}

impl DeliveryRoutes {
    /// Load from disk, returning default if absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    /// Persist to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize delivery routes")?;
        std::fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Set a delivery route for a webhook name.
    pub fn set_route(&mut self, webhook_name: &str, platform: &str, channel_id: &str) {
        self.routes.insert(
            webhook_name.to_string(),
            DeliveryTarget {
                platform: platform.to_string(),
                channel_id: channel_id.to_string(),
            },
        );
    }

    /// Remove a delivery route.
    pub fn remove_route(&mut self, webhook_name: &str) -> bool {
        self.routes.remove(webhook_name).is_some()
    }
}

/// Format a raw webhook payload into a human-readable message for chat delivery.
///
/// The payload is typically a JSON body from an inbound webhook request.
/// This function formats it into a readable string suitable for chat messages.
pub fn format_webhook_payload(
    webhook_name: &str,
    event_type: Option<&str>,
    payload: &str,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("📨 Webhook: {webhook_name}"));

    if let Some(event) = event_type {
        lines.push(format!("Event: {event}"));
    }

    lines.push(String::new());

    // Try to pretty-print JSON payloads.
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) {
        // Try to extract a short summary from common webhook formats.
        if let Some(summary) = extract_summary(&json) {
            lines.push(summary);
            lines.push(String::new());
        }
        match serde_json::to_string_pretty(&json) {
            Ok(pretty) => {
                // Truncate to ~3500 chars to stay within Telegram message limits.
                let truncated = truncate_with_ellipsis(&pretty, 3500);
                lines.push(format!("```{}", truncated));
                if pretty.len() > 3500 {
                    lines.push("```".to_string());
                    lines.push("_(payload truncated)_".to_string());
                }
            }
            Err(_) => lines.push(payload.to_string()),
        }
    } else {
        // Non-JSON payload — just truncate.
        let truncated = truncate_with_ellipsis(payload, 3500);
        lines.push(truncated);
    }

    lines.join("\n")
}

/// Try to extract a one-line summary from a common webhook JSON structure.
fn extract_summary(json: &serde_json::Value) -> Option<String> {
    // GitHub-style: push event with commits.
    if let Some(head_commit) = json.get("head_commit") {
        let msg = head_commit.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let author = head_commit
            .get("author")
            .and_then(|a| a.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Some(format!("Commit by {author}: {msg}"));
    }

    // Generic: look for "message" or "text" at the top level.
    if let Some(msg) = json.get("message").and_then(|v| v.as_str()) {
        return Some(msg.to_string());
    }
    if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
        return Some(text.to_string());
    }

    None
}

/// Truncate a string with ellipsis indicator.
fn truncate_with_ellipsis(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a safe char boundary.
        let mut end = max_len;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

/// Deliver a webhook payload directly to a target channel, bypassing the
/// agent/LLM loop.  Dispatches to the appropriate platform adapter.
///
/// # Arguments
/// * `platform` - "telegram", "discord", or "slack"
/// * `channel_id` - The chat/channel ID to send to
/// * `message` - The formatted message to send
pub fn deliver_to_platform(platform: &str, channel_id: &str, message: &str) -> Result<()> {
    match platform {
        "telegram" => deliver_to_telegram(channel_id, message),
        "discord" => deliver_to_discord(channel_id, message),
        "slack" => deliver_to_slack(channel_id, message),
        other => bail!("unsupported platform for direct delivery: {other}"),
    }
}

/// High-level entry point: deliver a webhook payload to a target channel.
///
/// Formats the payload and sends it via the messaging router.
/// The caller (dashboard webhook handler) should call this when
/// `webhook.direct_delivery == true`.
pub fn deliver_webhook_to_channel(
    webhook_name: &str,
    event_type: Option<&str>,
    payload: &str,
    platform: &str,
    channel_id: &str,
) -> DeliveryResult {
    let message = format_webhook_payload(webhook_name, event_type, payload);

    match deliver_to_platform(platform, channel_id, &message) {
        Ok(()) => DeliveryResult {
            platform: platform.to_string(),
            channel_id: channel_id.to_string(),
            success: true,
            error: None,
        },
        Err(e) => {
            let error_msg = e.to_string();
            log::warn!(
                "direct delivery failed for webhook '{webhook_name}' to {platform}:{channel_id}: {error_msg}"
            );
            DeliveryResult {
                platform: platform.to_string(),
                channel_id: channel_id.to_string(),
                success: false,
                error: Some(error_msg),
            }
        }
    }
}

/// Resolve the delivery target for a webhook name using the route map.
/// Falls back to the primary Telegram chat if no route is configured.
pub fn resolve_delivery_target(routes_path: &Path, webhook_name: &str) -> Result<DeliveryTarget> {
    let routes = DeliveryRoutes::load(routes_path)?;

    if let Some(target) = routes.routes.get(webhook_name) {
        return Ok(target.clone());
    }

    // Fall back to the primary Telegram chat.
    let bot = crate::messaging::TelegramBot::from_env()?;
    let chat_id = bot
        .primary_chat_id()
        .context("no delivery route configured and no primary Telegram chat available")?;

    Ok(DeliveryTarget {
        platform: DEFAULT_PLATFORM.to_string(),
        channel_id: chat_id.to_string(),
    })
}

fn deliver_to_telegram(channel_id: &str, message: &str) -> Result<()> {
    use crate::messaging::TelegramBot;
    let bot = TelegramBot::from_env()?;
    let chat_id = channel_id
        .parse::<i64>()
        .context("invalid Telegram chat ID for direct delivery")?;
    bot.send_message(chat_id, message)
}

fn deliver_to_discord(channel_id: &str, message: &str) -> Result<()> {
    #[cfg(feature = "discord")]
    {
        use crate::messaging::DiscordClient;
        let client = DiscordClient::from_env()?;
        client.send_message(channel_id, message)?;
        Ok(())
    }
    #[cfg(not(feature = "discord"))]
    {
        let _ = (channel_id, message);
        bail!("discord feature is not enabled — cannot deliver webhook directly")
    }
}

fn deliver_to_slack(channel_id: &str, message: &str) -> Result<()> {
    #[cfg(feature = "slack")]
    {
        use crate::messaging::SlackClient;
        let client = SlackClient::from_env()?;
        client.post_message(channel_id, message)?;
        Ok(())
    }
    #[cfg(not(feature = "slack"))]
    {
        let _ = (channel_id, message);
        bail!("slack feature is not enabled — cannot deliver webhook directly")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_simple_json_payload() {
        let msg = format_webhook_payload(
            "ci-cd",
            Some("deploy"),
            r#"{"status":"success","branch":"main"}"#,
        );
        assert!(msg.contains("ci-cd"));
        assert!(msg.contains("deploy"));
        assert!(msg.contains("success"));
    }

    #[test]
    fn format_github_push_payload() {
        let payload = r#"{
            "ref": "refs/heads/main",
            "head_commit": {
                "message": "Fix bug",
                "author": {"username": "alice"}
            }
        }"#;
        let msg = format_webhook_payload("github", None, payload);
        assert!(msg.contains("Commit by alice: Fix bug"));
    }

    #[test]
    fn format_non_json_payload() {
        let msg = format_webhook_payload("raw", None, "plain text payload");
        assert!(msg.contains("plain text payload"));
    }

    #[test]
    fn truncate_long_payload() {
        let long = "x".repeat(5000);
        let truncated = truncate_with_ellipsis(&long, 100);
        assert!(truncated.len() < 110);
        assert!(truncated.ends_with('…'));
    }

    #[test]
    fn delivery_routes_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("routes.json");

        let mut routes = DeliveryRoutes::default();
        routes.set_route("ci", "telegram", "123456");
        routes.save(&path).unwrap();

        let loaded = DeliveryRoutes::load(&path).unwrap();
        assert_eq!(loaded.routes.len(), 1);
        let target = loaded.routes.get("ci").unwrap();
        assert_eq!(target.platform, "telegram");
        assert_eq!(target.channel_id, "123456");
    }

    #[test]
    fn delivery_result_on_unknown_platform() {
        let result = deliver_webhook_to_channel("test", None, "hello", "irc", "#chan");
        assert!(!result.success);
        assert!(result.error.unwrap().contains("unsupported platform"));
    }
}
