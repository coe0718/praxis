use std::{collections::HashMap, fs, path::Path, time::Duration};

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

        let payload = WebhookPayload { content: text, username };

        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .context("failed to POST to Discord webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Discord webhook POST failed with {status}: {safe_body}");
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
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Discord send_message failed with {status}: {safe_body}");
        }

        response.json().context("failed to parse Discord message response")
    }

    /// Poll one or more watched channels for new messages since the last run.
    ///
    /// Requires `PRAXIS_DISCORD_BOT_TOKEN` and `PRAXIS_DISCORD_CHANNEL_IDS`
    /// (comma-separated channel IDs).  Optionally gate by `PRAXIS_DISCORD_ALLOWED_USER_IDS`.
    ///
    /// Offset state is persisted to `state_path` (`discord_state.json`).
    pub fn poll_once(
        &self,
        state_path: &Path,
        allowed_user_ids: &[String],
    ) -> Result<Vec<DiscordPollMessage>> {
        let channel_ids = parse_channel_ids()?;
        let token = self
            .bot_token
            .as_deref()
            .context("PRAXIS_DISCORD_BOT_TOKEN is required for polling")?;

        let mut state = load_discord_state(state_path).unwrap_or_default();
        let mut accepted = Vec::new();

        for channel_id in &channel_ids {
            let after = state.last_message_ids.get(channel_id).cloned();
            let messages = self.fetch_channel_messages(token, channel_id, after.as_deref())?;

            for msg in messages {
                if msg.content.trim().is_empty() {
                    continue;
                }
                // Update the per-channel offset (Discord message IDs are snowflakes — lexically ordered).
                let current_max = state
                    .last_message_ids
                    .get(channel_id)
                    .cloned()
                    .unwrap_or_else(|| "0".to_string());
                if msg.id > current_max {
                    state.last_message_ids.insert(channel_id.clone(), msg.id.clone());
                }

                let author_id = msg.author_id().unwrap_or_default();

                // Gate by allowed user IDs if the list is non-empty.
                if !allowed_user_ids.is_empty() && !allowed_user_ids.iter().any(|a| a == &author_id)
                {
                    continue;
                }

                accepted.push(DiscordPollMessage {
                    channel_id: channel_id.clone(),
                    message_id: msg.id,
                    author_id,
                    content: msg.content,
                });
            }
        }

        save_discord_state(state_path, &state)?;
        Ok(accepted)
    }

    fn fetch_channel_messages(
        &self,
        token: &str,
        channel_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RawDiscordMessage>> {
        let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");
        let mut req = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {token}"))
            .query(&[("limit", "100")]);
        if let Some(after_id) = after {
            req = req.query(&[("after", after_id)]);
        }
        let resp = req.send().context("failed to GET Discord channel messages")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Discord channel messages GET failed with {status}: {safe_body}");
        }
        resp.json::<Vec<RawDiscordMessage>>()
            .context("failed to parse Discord channel messages")
    }
}

/// A message received during a Discord poll cycle.
#[derive(Debug, Clone)]
pub struct DiscordPollMessage {
    pub channel_id: String,
    pub message_id: String,
    pub author_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct RawDiscordMessage {
    id: String,
    content: String,
    #[serde(default)]
    author: Option<RawDiscordAuthor>,
}

impl RawDiscordMessage {
    fn author_id(&self) -> Option<String> {
        self.author.as_ref().map(|a| a.id.clone())
    }
}

#[derive(Debug, Deserialize)]
struct RawDiscordAuthor {
    id: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct DiscordPollState {
    /// Last seen message ID per channel (Discord snowflakes are lexically ordered).
    #[serde(default)]
    last_message_ids: HashMap<String, String>,
}

fn parse_channel_ids() -> Result<Vec<String>> {
    let raw = std::env::var("PRAXIS_DISCORD_CHANNEL_IDS")
        .context("PRAXIS_DISCORD_CHANNEL_IDS is required for Discord polling")?;
    Ok(raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

pub fn parse_allowed_user_ids() -> Vec<String> {
    std::env::var("PRAXIS_DISCORD_ALLOWED_USER_IDS")
        .map(|raw| raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

fn load_discord_state(path: &Path) -> Result<DiscordPollState> {
    if !path.exists() {
        return Ok(DiscordPollState::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("invalid discord state in {}", path.display()))
}

fn save_discord_state(path: &Path, state: &DiscordPollState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state).context("failed to serialize discord state")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}
