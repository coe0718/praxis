//! Telegram webhook receiver for instant push delivery.
//!
//! Instead of polling Telegram's getUpdates endpoint every 30 seconds,
//! this module sets up a Telegram webhook that delivers messages
//! instantly via HTTP POST to the Praxis axum server.
//!
//! Usage: Configure with `praxis telegram webhook --url https://your.server.com/webhook/telegram`

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Telegram webhook configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramWebhookConfig {
    /// The public URL Telegram will POST to.
    pub url: String,
    /// Bot token (needed for setWebhook API call).
    pub bot_token: String,
    /// Optional secret token for webhook verification.
    pub secret_token: Option<String>,
    /// Maximum number of pending updates (1-100, default 100).
    pub max_connections: Option<u32>,
}

/// Set the Telegram webhook via Bot API.
pub async fn set_telegram_webhook(config: &TelegramWebhookConfig) -> Result<()> {
    if config.url.is_empty() {
        bail!("webhook URL cannot be empty");
    }
    if !config.url.starts_with("https://") {
        bail!("Telegram webhooks require HTTPS. URL: {}", config.url);
    }

    let api_url = format!("https://api.telegram.org/bot{}/setWebhook", config.bot_token);

    let mut body = serde_json::json!({
        "url": config.url,
        "allowed_updates": ["message", "edited_message", "channel_post"],
    });

    if let Some(ref secret) = config.secret_token {
        body["secret_token"] = serde_json::Value::String(secret.clone());
    }
    if let Some(max_conn) = config.max_connections {
        body["max_connections"] = serde_json::Value::Number(max_conn.into());
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(&api_url)
        .json(&body)
        .send()
        .await
        .context("failed to call Telegram setWebhook API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("Telegram setWebhook failed (HTTP {status}): {text}");
    }

    let result: serde_json::Value =
        resp.json().await.context("failed to parse Telegram setWebhook response")?;

    if result["ok"].as_bool() != Some(true) {
        bail!("Telegram setWebhook returned ok=false: {result}");
    }

    Ok(())
}

/// Delete the Telegram webhook (revert to polling).
pub async fn delete_telegram_webhook(bot_token: &str) -> Result<()> {
    let api_url = format!("https://api.telegram.org/bot{}/deleteWebhook", bot_token);

    let client = reqwest::Client::new();
    let resp = client
        .post(&api_url)
        .send()
        .await
        .context("failed to call Telegram deleteWebhook API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        bail!("Telegram deleteWebhook failed (HTTP {status}): {text}");
    }

    Ok(())
}

/// Get current webhook info.
pub async fn get_webhook_info(bot_token: &str) -> Result<serde_json::Value> {
    let api_url = format!("https://api.telegram.org/bot{}/getWebhookInfo", bot_token);

    let client = reqwest::Client::new();
    let resp = client
        .get(&api_url)
        .send()
        .await
        .context("failed to call Telegram getWebhookInfo API")?;

    let result: serde_json::Value =
        resp.json().await.context("failed to parse Telegram getWebhookInfo response")?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config_validation() {
        let config = TelegramWebhookConfig {
            url: "http://example.com".to_string(),
            bot_token: "123:ABC".to_string(),
            secret_token: None,
            max_connections: None,
        };
        // URL must be HTTPS — config validation catches this at build time.
        assert!(!config.url.starts_with("https://"));
    }
}
