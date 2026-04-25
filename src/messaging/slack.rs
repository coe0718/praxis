use std::{collections::HashMap, fs, path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Thin Slack client.
///
/// Set `PRAXIS_SLACK_BOT_TOKEN` (xoxb-…) for the Web API.
/// Set `PRAXIS_SLACK_WEBHOOK_URL` for an incoming webhook (no token needed).
pub struct SlackClient {
    client: Client,
    bot_token: Option<String>,
    webhook_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct WebhookPayload<'a> {
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct PostMessagePayload<'a> {
    channel: &'a str,
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct SlackApiResponse {
    ok: bool,
    error: Option<String>,
    ts: Option<String>,
}

/// Inbound event received via the `/webhook/slack` Events API endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackEvent {
    /// "url_verification" or "event_callback"
    #[serde(rename = "type")]
    pub event_type: String,
    /// Only present for url_verification challenges.
    pub challenge: Option<String>,
    pub event: Option<SlackEventPayload>,
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlackEventPayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub user: Option<String>,
    pub text: Option<String>,
    pub channel: Option<String>,
    pub ts: Option<String>,
}

impl SlackClient {
    pub fn from_env() -> Result<Self> {
        let bot_token = std::env::var("PRAXIS_SLACK_BOT_TOKEN").ok();
        let webhook_url = std::env::var("PRAXIS_SLACK_WEBHOOK_URL").ok();

        if bot_token.is_none() && webhook_url.is_none() {
            bail!("Slack requires either PRAXIS_SLACK_BOT_TOKEN or PRAXIS_SLACK_WEBHOOK_URL");
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
        let has_token = std::env::var("PRAXIS_SLACK_BOT_TOKEN").is_ok();
        let has_webhook = std::env::var("PRAXIS_SLACK_WEBHOOK_URL").is_ok();
        if !has_token && !has_webhook {
            bail!("Slack requires PRAXIS_SLACK_BOT_TOKEN or PRAXIS_SLACK_WEBHOOK_URL");
        }
        Ok(())
    }

    /// Send a message via an incoming webhook URL.
    pub fn send_webhook(&self, text: &str) -> Result<()> {
        let url = self
            .webhook_url
            .as_deref()
            .context("PRAXIS_SLACK_WEBHOOK_URL is not set — cannot send via webhook")?;

        let response = self
            .client
            .post(url)
            .json(&WebhookPayload { text })
            .send()
            .context("failed to POST to Slack webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Slack webhook POST failed with {status}: {safe_body}");
        }

        Ok(())
    }

    /// Post a message to a channel via `chat.postMessage` (requires bot token).
    pub fn post_message(&self, channel: &str, text: &str) -> Result<String> {
        let token = self
            .bot_token
            .as_deref()
            .context("PRAXIS_SLACK_BOT_TOKEN is not set — cannot use the Web API")?;

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {token}"))
            .json(&PostMessagePayload { channel, text })
            .send()
            .context("failed to POST to Slack chat.postMessage")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Slack chat.postMessage failed with {status}: {safe_body}");
        }

        let parsed: SlackApiResponse =
            response.json().context("failed to parse Slack chat.postMessage response")?;

        if !parsed.ok {
            bail!("Slack chat.postMessage error: {}", parsed.error.as_deref().unwrap_or("unknown"));
        }

        Ok(parsed.ts.unwrap_or_default())
    }

    /// Poll one or more watched channels for new messages since the last run.
    ///
    /// Requires `PRAXIS_SLACK_BOT_TOKEN` and `PRAXIS_SLACK_CHANNEL_IDS`
    /// (comma-separated channel IDs or names, e.g. `C01234ABCD,C09876WXYZ`).
    /// Optionally gate by `PRAXIS_SLACK_ALLOWED_USER_IDS`.
    ///
    /// Offset state is persisted to `state_path` (`slack_state.json`).
    pub fn poll_once(
        &self,
        state_path: &Path,
        allowed_user_ids: &[String],
    ) -> Result<Vec<SlackPollMessage>> {
        let token = self
            .bot_token
            .as_deref()
            .context("PRAXIS_SLACK_BOT_TOKEN is required for polling")?;

        let channel_ids = parse_slack_channel_ids()?;
        let mut state = load_slack_state(state_path).unwrap_or_default();
        let mut accepted = Vec::new();

        for channel_id in &channel_ids {
            let oldest = state.last_ts.get(channel_id).cloned();
            let messages = self.fetch_history(token, channel_id, oldest.as_deref())?;

            for msg in messages {
                if msg.text.as_deref().unwrap_or("").trim().is_empty() {
                    continue;
                }
                // Update per-channel offset.
                let ts = msg.ts.clone();
                let current = state.last_ts.get(channel_id).cloned().unwrap_or_default();
                if ts > current {
                    state.last_ts.insert(channel_id.clone(), ts.clone());
                }

                let user = msg.user.unwrap_or_default();
                if !allowed_user_ids.is_empty() && !allowed_user_ids.iter().any(|a| a == &user) {
                    continue;
                }

                accepted.push(SlackPollMessage {
                    channel_id: channel_id.clone(),
                    ts,
                    user_id: user,
                    text: msg.text.unwrap_or_default(),
                });
            }
        }

        save_slack_state(state_path, &state)?;
        Ok(accepted)
    }

    fn fetch_history(
        &self,
        token: &str,
        channel_id: &str,
        oldest: Option<&str>,
    ) -> Result<Vec<RawSlackMessage>> {
        let mut req = self
            .client
            .get("https://slack.com/api/conversations.history")
            .header("Authorization", format!("Bearer {token}"))
            .query(&[("channel", channel_id), ("limit", "100")]);
        if let Some(ts) = oldest {
            req = req.query(&[("oldest", ts)]);
        }
        let resp = req.send().context("failed to GET Slack conversations.history")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let safe_body = body.chars().take(200).collect::<String>();
            bail!("Slack conversations.history failed with {status}: {safe_body}");
        }
        let envelope: SlackHistoryEnvelope =
            resp.json().context("failed to parse Slack conversations.history")?;
        if !envelope.ok {
            bail!(
                "Slack conversations.history error: {}",
                envelope.error.as_deref().unwrap_or("unknown")
            );
        }
        Ok(envelope.messages.unwrap_or_default())
    }
}

/// A message received during a Slack poll cycle.
#[derive(Debug, Clone)]
pub struct SlackPollMessage {
    pub channel_id: String,
    pub ts: String,
    pub user_id: String,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct RawSlackMessage {
    ts: String,
    text: Option<String>,
    user: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryEnvelope {
    ok: bool,
    error: Option<String>,
    messages: Option<Vec<RawSlackMessage>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SlackPollState {
    /// Last seen message timestamp per channel.
    #[serde(default)]
    last_ts: HashMap<String, String>,
}

fn parse_slack_channel_ids() -> Result<Vec<String>> {
    let raw = std::env::var("PRAXIS_SLACK_CHANNEL_IDS")
        .context("PRAXIS_SLACK_CHANNEL_IDS is required for Slack polling")?;
    Ok(raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

pub fn parse_allowed_user_ids() -> Vec<String> {
    std::env::var("PRAXIS_SLACK_ALLOWED_USER_IDS")
        .map(|raw| raw.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default()
}

fn load_slack_state(path: &Path) -> Result<SlackPollState> {
    if !path.exists() {
        return Ok(SlackPollState::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("invalid slack state in {}", path.display()))
}

fn save_slack_state(path: &Path, state: &SlackPollState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state).context("failed to serialize slack state")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}
