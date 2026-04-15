use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Thin Discord REST client.
///
/// Set `PRAXIS_DISCORD_BOT_TOKEN` to a Discord bot token.
/// Set `PRAXIS_DISCORD_WEBHOOK_URL` to post via an incoming webhook (no token needed).
pub struct DiscordClient {
    client: Client,
    /// Bot token — used for the REST API (send DMs, channel messages, etc.)
    bot_token: Option<String>,
    /// Incoming webhook URL — simpler than the REST API.
    webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct WebhookPayload<'a> {
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct ChannelMessagePayload<'a> {
    content: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub content: String,
}

/// Inbound event received via the `/webhook/discord` endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct DiscordInteraction {
    /// Interaction type: 1 = PING, 2 = APPLICATION_COMMAND, 3 = MESSAGE_COMPONENT
    #[serde(rename = "type")]
    pub interaction_type: u8,
    pub data: Option<DiscordInteractionData>,
    pub channel_id: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordInteractionData {
    pub name: Option<String>,
    pub options: Option<Vec<DiscordCommandOption>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordCommandOption {
    pub name: String,
    pub value: Option<serde_json::Value>,
}

impl DiscordClient {
    pub fn from_env() -> Result<Self> {
        let bot_token = std::env::var("PRAXIS_DISCORD_BOT_TOKEN").ok();
        let webhook_url = std::env::var("PRAXIS_DISCORD_WEBHOOK_URL").ok();

        if bot_token.is_none() && webhook_url.is_none() {
            bail!("Discord requires either PRAXIS_DISCORD_BOT_TOKEN or PRAXIS_DISCORD_WEBHOOK_URL");
        }

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .context("failed to build HTTP client")?,
            bot_token,
            webhook_url,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_token = std::env::var("PRAXIS_DISCORD_BOT_TOKEN").is_ok();
        let has_webhook = std::env::var("PRAXIS_DISCORD_WEBHOOK_URL").is_ok();
        if !has_token && !has_webhook {
            bail!("Discord requires PRAXIS_DISCORD_BOT_TOKEN or PRAXIS_DISCORD_WEBHOOK_URL");
        }
        Ok(())
    }

    /// Send a message via an incoming webhook URL.
    pub fn send_webhook(&self, text: &str, username: Option<&str>) -> Result<()> {
        let url = self
            .webhook_url
            .as_deref()
            .context("PRAXIS_DISCORD_WEBHOOK_URL is not set — cannot send via webhook")?;

        let payload = WebhookPayload {
            content: text,
            username,
        };

        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .context("failed to POST to Discord webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Discord webhook POST failed with {status}: {body}");
        }

        Ok(())
    }

    /// Send a message to a channel via the Discord REST API (requires bot token).
    pub fn send_message(&self, channel_id: &str, text: &str) -> Result<DiscordMessage> {
        let token = self
            .bot_token
            .as_deref()
            .context("PRAXIS_DISCORD_BOT_TOKEN is not set — cannot use the REST API")?;

        let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");

        let payload = ChannelMessagePayload { content: text };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {token}"))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .context("failed to POST to Discord messages endpoint")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Discord send_message failed with {status}: {body}");
        }

        response
            .json()
            .context("failed to parse Discord message response")
    }
}
