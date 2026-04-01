use std::{fs, path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TelegramBot {
    client: Client,
    token: String,
    allowed_chat_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramInboundMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramInboundMessage {
    pub chat: TelegramChat,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
}

impl TelegramBot {
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("PRAXIS_TELEGRAM_BOT_TOKEN")
            .context("PRAXIS_TELEGRAM_BOT_TOKEN is required for Telegram support")?;
        let allowed_chat_ids = parse_allowed_chat_ids()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("failed to build Telegram HTTP client")?;
        Ok(Self {
            client,
            token,
            allowed_chat_ids,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let _ = std::env::var("PRAXIS_TELEGRAM_BOT_TOKEN")
            .context("PRAXIS_TELEGRAM_BOT_TOKEN is required for Telegram support")?;
        parse_allowed_chat_ids().map(|_| ())
    }

    pub fn poll_once(&self, state_path: &Path) -> Result<Vec<TelegramMessage>> {
        let offset = load_offset(state_path).unwrap_or(0);
        let updates = self.fetch_updates(offset + 1)?;
        let next_offset = updates
            .iter()
            .map(|update| update.update_id)
            .max()
            .unwrap_or(offset);
        if next_offset > offset {
            save_offset(state_path, next_offset)?;
        }
        Ok(filter_messages(&self.allowed_chat_ids, &updates))
    }

    pub fn send_message(&self, chat_id: i64, text: &str) -> Result<()> {
        self.client
            .post(api_url(&self.token, "sendMessage"))
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
            .send()
            .context("failed to send Telegram message")?
            .error_for_status()
            .context("Telegram sendMessage returned an error")?;
        Ok(())
    }

    fn fetch_updates(&self, offset: i64) -> Result<Vec<TelegramUpdate>> {
        let response = self
            .client
            .get(api_url(&self.token, "getUpdates"))
            .query(&[("timeout", "1"), ("offset", &offset.to_string())])
            .send()
            .context("failed to poll Telegram updates")?
            .error_for_status()
            .context("Telegram getUpdates returned an error")?;

        let envelope = response
            .json::<TelegramEnvelope>()
            .context("failed to parse Telegram updates")?;
        Ok(envelope.result)
    }
}

fn parse_allowed_chat_ids() -> Result<Vec<i64>> {
    let raw = std::env::var("PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS")
        .context("PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS is required for Telegram support")?;
    let ids = raw
        .split(',')
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<i64>()
                .with_context(|| format!("invalid Telegram chat id {value}"))
        })
        .collect::<Result<Vec<_>>>()?;
    if ids.is_empty() {
        bail!("PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS must contain at least one chat id");
    }
    Ok(ids)
}

fn api_url(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}

fn load_offset(path: &Path) -> Result<i64> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read Telegram state {}", path.display()))?;
    let state = serde_json::from_str::<TelegramState>(&raw)
        .with_context(|| format!("invalid Telegram state in {}", path.display()))?;
    Ok(state.last_update_id)
}

fn save_offset(path: &Path, last_update_id: i64) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let state = TelegramState { last_update_id };
    let raw = serde_json::to_string_pretty(&state).context("failed to serialize Telegram state")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}

fn filter_messages(allowed_chat_ids: &[i64], updates: &[TelegramUpdate]) -> Vec<TelegramMessage> {
    updates
        .iter()
        .filter_map(|update| update.message.as_ref())
        .filter_map(|message| {
            let text = message.text.as_deref()?.trim();
            if text.is_empty() || !allowed_chat_ids.contains(&message.chat.id) {
                return None;
            }
            Some(TelegramMessage {
                chat_id: message.chat.id,
                text: text.to_string(),
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct TelegramEnvelope {
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TelegramState {
    last_update_id: i64,
}
