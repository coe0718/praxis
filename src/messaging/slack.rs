use std::time::Duration;

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
            bail!("Slack webhook POST failed with {status}: {body}");
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
            bail!("Slack chat.postMessage failed with {status}: {body}");
        }

        let parsed: SlackApiResponse = response
            .json()
            .context("failed to parse Slack chat.postMessage response")?;

        if !parsed.ok {
            bail!(
                "Slack chat.postMessage error: {}",
                parsed.error.as_deref().unwrap_or("unknown")
            );
        }

        Ok(parsed.ts.unwrap_or_default())
    }
}
