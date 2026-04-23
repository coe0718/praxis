use std::{fs, path::Path, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{
    bus::{BusEvent, MessageBus},
    messaging::{ActivationStore, TypingIndicator, pairing::PairingStore},
};

#[derive(Debug, Clone)]
pub struct TelegramBot {
    client: Client,
    token: String,
    allowed_chat_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelegramMessage {
    pub chat_id: i64,
    pub sender_id: i64,
    pub text: String,
    /// True if the message was a direct reply to another message.
    pub is_reply: bool,
    /// True if the message contains a @mention entity directed at the bot.
    pub is_mention: bool,
    /// True if the chat is a private (1:1) conversation.
    pub is_private: bool,
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
    pub from: Option<TelegramUser>,
    /// Presence indicates a threaded reply; content is not inspected.
    pub reply_to_message: Option<serde_json::Value>,
    pub entities: Option<Vec<TelegramMessageEntity>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    /// "private", "group", "supergroup", or "channel".
    #[serde(default)]
    pub r#type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessageEntity {
    /// Entity type, e.g. "mention", "bot_command", "text_mention".
    pub r#type: String,
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

    /// Poll for new messages, gate them through the activation store, and
    /// publish accepted messages onto the message bus.
    ///
    /// `pairing_path` points to `sender_pairing.json`.  Messages from unknown
    /// chats trigger a one-time code flow rather than being silently dropped.
    ///
    /// # Concurrency
    /// This method uses an advisory lock file to prevent concurrent poll cycles
    /// from replaying or losing messages.
    pub fn poll_once(
        &self,
        state_path: &Path,
        pairing_path: &Path,
        bus: &dyn MessageBus,
        activation: &ActivationStore,
    ) -> Result<Vec<TelegramMessage>> {
        let _lock = acquire_poll_lock(state_path)?;
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
        filter_messages(
            self,
            &self.allowed_chat_ids,
            pairing_path,
            &updates,
            bus,
            activation,
        )
    }

    /// Return the first configured chat ID — the primary operator chat.
    pub fn primary_chat_id(&self) -> Option<i64> {
        self.allowed_chat_ids.first().copied()
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

    /// Emit a "typing…" presence indicator in the given chat.
    /// Telegram cancels it automatically after 5 seconds or on message send.
    pub fn send_typing(&self, chat_id: i64) -> Result<()> {
        self.client
            .post(api_url(&self.token, "sendChatAction"))
            .json(&serde_json::json!({ "chat_id": chat_id, "action": "typing" }))
            .send()
            .context("failed to send Telegram chat action")?
            .error_for_status()
            .context("Telegram sendChatAction returned an error")?;
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

impl TypingIndicator for TelegramBot {
    fn begin(&self, conversation_id: &str) -> anyhow::Result<()> {
        if let Ok(chat_id) = conversation_id.parse::<i64>() {
            self.send_typing(chat_id)?;
        }
        Ok(())
    }

    fn end(&self, _conversation_id: &str) -> anyhow::Result<()> {
        // Telegram cancels typing automatically on message delivery.
        Ok(())
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
    // Prevent lost updates from concurrent poll_once calls: never overwrite
    // a higher or equal offset that another process may have already saved.
    if let Ok(current) = load_offset(path) && last_update_id <= current {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let state = TelegramState { last_update_id };
    let raw = serde_json::to_string_pretty(&state).context("failed to serialize Telegram state")?;
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, raw)
        .with_context(|| format!("failed to write {}", temp_path.display()))?;
    fs::rename(&temp_path, path)
        .with_context(|| format!("failed to rename {} to {}", temp_path.display(), path.display()))
}

/// Advisory lock file to prevent concurrent `poll_once` calls.
/// Uses `create_new` for atomic creation.  If a stale lock (older than 5 min)
/// is found it is removed and re-acquired.
struct PollLock {
    path: std::path::PathBuf,
}

impl Drop for PollLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn acquire_poll_lock(state_path: &Path) -> Result<PollLock> {
    let lock_path = state_path.with_extension("lock");
    // If a lock exists and is stale, remove it.
    if let Ok(meta) = fs::metadata(&lock_path)
        && let Ok(modified) = meta.modified()
        && std::time::SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            > Duration::from_secs(300)
    {
        let _ = fs::remove_file(&lock_path);
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(_) => Ok(PollLock { path: lock_path }),
        Err(e) => anyhow::bail!(
            "another Telegram poll is already in progress (lock file exists): {e}"
        ),
    }
}

fn has_mention_entity(entities: &Option<Vec<TelegramMessageEntity>>) -> bool {
    entities.as_deref().is_some_and(|ents| {
        ents.iter()
            .any(|e| e.r#type == "mention" || e.r#type == "text_mention")
    })
}

/// Filter raw updates down to accepted messages, applying the activation gate
/// for group chats and publishing each accepted message onto the bus.
///
/// Unknown chat IDs go through the sender pairing flow instead of being
/// dropped silently.
fn filter_messages(
    bot: &TelegramBot,
    allowed_chat_ids: &[i64],
    pairing_path: &Path,
    updates: &[TelegramUpdate],
    bus: &dyn MessageBus,
    activation: &ActivationStore,
) -> Result<Vec<TelegramMessage>> {
    let mut accepted = Vec::new();
    let mut pairing = PairingStore::load(pairing_path)?;
    let mut pairing_dirty = false;

    for update in updates {
        let Some(message) = update.message.as_ref() else {
            continue;
        };
        let Some(text) = message.text.as_deref() else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() {
            continue;
        }

        let chat_id = message.chat.id;
        let in_static_allow_list = allowed_chat_ids.contains(&chat_id);
        let in_pairing_allow_list = pairing.is_approved(chat_id);

        if !in_static_allow_list && !in_pairing_allow_list {
            // Unknown chat — run the pairing flow.
            if !pairing.is_pending(chat_id) {
                let code = pairing.initiate(chat_id, text);
                pairing_dirty = true;
                // Notify the operator via the first allowed chat.
                if let Some(&operator_chat) = allowed_chat_ids.first() {
                    let notice = format!(
                        "⚠️ Unknown sender (chat {chat_id}) wants access.\n\
                         Their message has been queued.\n\
                         To approve, send: /approve-sender {code}"
                    );
                    // Best-effort — don't fail the whole poll if this errors.
                    if let Err(e) = bot.send_message(operator_chat, &notice) {
                        log::warn!("sender pairing notification failed: {e}");
                    }
                }
            }
            // Already pending — ignore until approved.
            continue;
        }

        let is_private = message.chat.r#type == "private" || message.chat.r#type.is_empty();
        let is_mention = has_mention_entity(&message.entities);
        let is_reply = message.reply_to_message.is_some();
        let conversation_id = chat_id.to_string();
        let sender_id = message.from.as_ref().map(|u| u.id).unwrap_or(0);

        // Private chats always pass through; groups obey the activation mode.
        if !is_private && !activation.should_respond(&conversation_id, is_mention, is_reply) {
            continue;
        }

        let event = BusEvent::new(
            "message",
            "telegram",
            &conversation_id,
            sender_id.to_string(),
            text,
        );
        bus.publish(&event)?;

        accepted.push(TelegramMessage {
            chat_id,
            sender_id,
            text: text.to_string(),
            is_reply,
            is_mention,
            is_private,
        });
    }

    if pairing_dirty {
        pairing.save(pairing_path)?;
    }

    Ok(accepted)
}

#[derive(Debug, Deserialize)]
struct TelegramEnvelope {
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TelegramState {
    last_update_id: i64,
}
