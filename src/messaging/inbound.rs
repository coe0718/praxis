//! Inbound message polling for Discord and Slack.
//!
//! Adds inbound polling loops so the daemon can receive messages
//! from Discord (via Gateway REST API) and Slack (via Web API),
//! publishing them as BusEvents for the agent to process.

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;

use crate::bus::BusEvent;

// ── Discord Inbound ──────────────────────────────────────────────────────────

/// Poll Discord for new messages since the last seen message ID.
///
/// Uses the Discord REST API `/channels/{id}/messages` endpoint.
/// Returns new messages as BusEvents.
pub fn poll_discord_messages(
    bot_token: &str,
    channel_id: &str,
    after_message_id: &str,
) -> Result<Vec<BusEvent>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build Discord HTTP client")?;

    let url = format!(
        "https://discord.com/api/v10/channels/{}/messages?after={}&limit=50",
        channel_id, after_message_id
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bot {}", bot_token))
        .send()
        .with_context(|| format!("poll Discord channel {}", channel_id))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("Discord API error {}: {}", status, &body[..body.len().min(200)]);
    }

    let messages: Vec<DiscordInboundMessage> = resp.json().context("parse Discord messages")?;

    let mut events = Vec::new();
    for msg in messages {
        log::debug!(
            "discord inbound: message {} from {} (#{}) in channel {}",
            msg.id,
            msg.author.username,
            msg.author.id,
            msg.channel_id
        );
        events.push(BusEvent {
            kind: "message".to_string(),
            source: "discord".to_string(),
            conversation_id: msg.channel_id.clone(),
            sender_id: msg.author.id.clone(),
            payload: msg.content.clone(),
            published_at: Utc::now(),
        });
    }

    Ok(events)
}

#[derive(Debug, Deserialize)]
struct DiscordInboundMessage {
    id: String,
    channel_id: String,
    content: String,
    author: DiscordAuthor,
}

#[derive(Debug, Deserialize)]
struct DiscordAuthor {
    id: String,
    username: String,
}

// ── Slack Inbound ────────────────────────────────────────────────────────────

/// Poll Slack for new messages using the conversations.history API.
pub fn poll_slack_messages(
    bot_token: &str,
    channel_id: &str,
    oldest_ts: &str,
) -> Result<Vec<BusEvent>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build Slack HTTP client")?;

    let url = format!(
        "https://slack.com/api/conversations.history?channel={}&oldest={}&limit=50",
        channel_id, oldest_ts
    );

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", bot_token))
        .send()
        .with_context(|| format!("poll Slack channel {}", channel_id))?;

    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("Slack API error {}", status);
    }

    let body: SlackHistoryResponse = resp.json().context("parse Slack history response")?;

    if !body.ok {
        anyhow::bail!("Slack API error: {:?}", body.error);
    }

    let mut events = Vec::new();
    for msg in body.messages.unwrap_or_default() {
        if let Some(ref ts) = msg.ts {
            log::debug!(
                "slack inbound: message ts={ts} from {}",
                msg.user.as_deref().unwrap_or("unknown")
            );
        }
        events.push(BusEvent {
            kind: "message".to_string(),
            source: "slack".to_string(),
            conversation_id: channel_id.to_string(),
            sender_id: msg.user.clone().unwrap_or_default(),
            payload: msg.text.clone(),
            published_at: Utc::now(),
        });
    }

    Ok(events)
}

#[derive(Debug, Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    error: Option<String>,
    messages: Option<Vec<SlackInboundMessage>>,
}

#[derive(Debug, Deserialize)]
struct SlackInboundMessage {
    text: String,
    user: Option<String>,
    ts: Option<String>,
}

// ── Inbound Poller ───────────────────────────────────────────────────────────

/// Configuration for the inbound polling loop.
#[derive(Debug, Clone)]
pub struct InboundPollConfig {
    /// Polling interval in seconds.
    pub interval_secs: u64,
    /// Discord channel IDs to poll.
    pub discord_channels: Vec<String>,
    /// Slack channel IDs to poll.
    pub slack_channels: Vec<String>,
}

impl Default for InboundPollConfig {
    fn default() -> Self {
        Self {
            interval_secs: 30,
            discord_channels: Vec::new(),
            slack_channels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbound_config_defaults() {
        let config = InboundPollConfig::default();
        assert_eq!(config.interval_secs, 30);
        assert!(config.discord_channels.is_empty());
        assert!(config.slack_channels.is_empty());
    }
}
