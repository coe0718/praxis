//! #24 `notify_on_complete`
//!
//! When a background task finishes, auto-notify the originating channel.
//!
//! The `NotificationQueue` tracks pending background tasks with their origin
//! channel. When a task completes, the caller looks up the channel and sends
//! a completion message via the messaging router.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Describes a background task that should notify a channel on completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTask {
    /// Unique task identifier (e.g. session ID, UUID, or descriptive name).
    pub task_id: String,
    /// The channel to notify when the task completes (e.g. Telegram chat ID,
    /// Discord channel ID).  Platform is inferred from the channel_id format
    /// or stored in `platform`.
    pub channel_id: String,
    /// Platform to deliver the notification on ("telegram", "discord", "slack").
    pub platform: String,
    /// Human-readable description of the task (included in the notification).
    pub description: String,
    /// When the task was registered.
    pub registered_at: DateTime<Utc>,
    /// When the task completed, if known.
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether the notification has been sent.
    #[serde(default)]
    pub notification_sent: bool,
}

/// In-memory thread-safe tracking of background tasks awaiting notification.
pub struct NotificationQueue {
    tasks: Mutex<HashMap<String, BackgroundTask>>,
}

impl NotificationQueue {
    /// Create an empty notification queue.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    /// Register a background task for eventual notification.
    pub fn register(
        &self,
        task_id: impl Into<String>,
        channel_id: impl Into<String>,
        platform: impl Into<String>,
        description: impl Into<String>,
    ) {
        let task = BackgroundTask {
            task_id: task_id.into(),
            channel_id: channel_id.into(),
            platform: platform.into(),
            description: description.into(),
            registered_at: Utc::now(),
            completed_at: None,
            notification_sent: false,
        };
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        log::info!(
            "notification registered: task={} channel={} platform={}",
            task.task_id,
            task.channel_id,
            task.platform,
        );
        tasks.insert(task.task_id.clone(), task);
    }

    /// Mark a task as completed and return the channel info for notification.
    ///
    /// Returns `Some(BackgroundTask)` if a matching task was found (the caller
    /// should then send the notification).  Returns `None` if the task ID is
    /// unknown or already notified.
    pub fn complete(&self, task_id: &str) -> Option<BackgroundTask> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get_mut(task_id) {
            task.completed_at = Some(Utc::now());
            if task.notification_sent {
                return None;
            }
            task.notification_sent = true;
            Some(task.clone())
        } else {
            None
        }
    }

    /// Return the number of pending (uncompleted) tasks.
    pub fn pending_count(&self) -> usize {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.values().filter(|t| t.completed_at.is_none()).count()
    }

    /// Purge completed-and-notified tasks from memory.
    pub fn purge_notified(&self) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.retain(|_, t| !t.notification_sent);
    }
}

impl Default for NotificationQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ── File-backed persistence ──────────────────────────────────────────────────
// For cross-process communication (CLI registers, daemon notifies).

/// Persistent file store for background task notifications.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationFileStore {
    #[serde(default)]
    pub tasks: Vec<BackgroundTask>,
}

impl NotificationFileStore {
    /// Load from disk, returning default if absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    /// Persist to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize notification store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Register a background task and persist.
    pub fn register_task(
        path: &Path,
        task_id: &str,
        channel_id: &str,
        platform: &str,
        description: &str,
    ) -> Result<()> {
        let mut store = Self::load(path)?;
        let task = BackgroundTask {
            task_id: task_id.to_string(),
            channel_id: channel_id.to_string(),
            platform: platform.to_string(),
            description: description.to_string(),
            registered_at: Utc::now(),
            completed_at: None,
            notification_sent: false,
        };
        store.tasks.push(task);
        store.save(path)
    }

    /// Mark a task as completed, persist, and return it for notification.
    pub fn complete_task(path: &Path, task_id: &str) -> Result<Option<BackgroundTask>> {
        let mut store = Self::load(path)?;
        let task = store.tasks.iter_mut().find(|t| t.task_id == task_id && !t.notification_sent);
        match task {
            Some(t) => {
                t.completed_at = Some(Utc::now());
                t.notification_sent = true;
                let result = t.clone();
                // Prune old notified tasks (keep last 100).
                if store.tasks.len() > 100 {
                    store.tasks.retain(|t| !t.notification_sent || t.completed_at.is_none());
                }
                store.save(path)?;
                Ok(Some(result))
            }
            None => Ok(None),
        }
    }
}

/// Send a task completion notification to the appropriate platform.
///
/// This function dispatches to the correct messaging adapter based on the
/// task's `platform` field. Errors are logged but not propagated (best-effort
/// notification).
pub fn send_completion_notification(task: &BackgroundTask, result_summary: &str) {
    let message = format!(
        "✅ Task complete: {}\n{}\nDuration: registered at {}",
        task.description,
        if result_summary.is_empty() {
            "Done."
        } else {
            result_summary
        },
        task.registered_at.format("%H:%M:%S UTC"),
    );

    match task.platform.as_str() {
        "telegram" => send_telegram_notification(&task.channel_id, &message),
        "discord" => send_discord_notification(&task.channel_id, &message),
        "slack" => send_slack_notification(&task.channel_id, &message),
        other => log::warn!("unknown platform '{}' for notification", other),
    }
}

fn send_telegram_notification(channel_id: &str, message: &str) {
    use crate::messaging::TelegramBot;

    let Ok(bot) = TelegramBot::from_env() else {
        log::warn!("telegram not configured — skipping completion notification");
        return;
    };
    let Ok(chat_id) = channel_id.parse::<i64>() else {
        log::warn!("invalid Telegram chat ID: {channel_id}");
        return;
    };
    if let Err(e) = bot.send_message(chat_id, message) {
        log::warn!("failed to send Telegram completion notification: {e}");
    }
}

fn send_discord_notification(channel_id: &str, message: &str) {
    #[cfg(feature = "discord")]
    {
        use crate::messaging::DiscordClient;
        let Ok(client) = DiscordClient::from_env() else {
            log::warn!("discord not configured — skipping completion notification");
            return;
        };
        if let Err(e) = client.send_message(channel_id, message) {
            log::warn!("failed to send Discord completion notification: {e}");
        }
    }
    #[cfg(not(feature = "discord"))]
    {
        let _ = (channel_id, message);
        log::warn!("discord feature disabled — skipping completion notification");
    }
}

fn send_slack_notification(channel_id: &str, message: &str) {
    #[cfg(feature = "slack")]
    {
        use crate::messaging::SlackClient;
        let Ok(client) = SlackClient::from_env() else {
            log::warn!("slack not configured — skipping completion notification");
            return;
        };
        if let Err(e) = client.post_message(channel_id, message) {
            log::warn!("failed to send Slack completion notification: {e}");
        }
    }
    #[cfg(not(feature = "slack"))]
    {
        let _ = (channel_id, message);
        log::warn!("slack feature disabled — skipping completion notification");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_queue_register_and_complete() {
        let q = NotificationQueue::new();
        assert_eq!(q.pending_count(), 0);

        q.register("task-1", "123456", "telegram", "Run tests");
        assert_eq!(q.pending_count(), 1);

        let task = q.complete("task-1").unwrap();
        assert_eq!(task.task_id, "task-1");
        assert_eq!(task.channel_id, "123456");
        assert!(task.notification_sent);
        assert!(task.completed_at.is_some());

        assert_eq!(q.pending_count(), 0);
    }

    #[test]
    fn notification_queue_complete_unknown_returns_none() {
        let q = NotificationQueue::new();
        assert!(q.complete("nonexistent").is_none());
    }

    #[test]
    fn notification_queue_double_complete_returns_none() {
        let q = NotificationQueue::new();
        q.register("task-1", "123", "telegram", "test");
        let _first = q.complete("task-1");
        let second = q.complete("task-1");
        assert!(second.is_none());
    }

    #[test]
    fn file_store_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("notifications.json");

        NotificationFileStore::register_task(
            &path,
            "bg-1",
            "999",
            "telegram",
            "Background analysis",
        )
        .unwrap();

        let task = NotificationFileStore::complete_task(&path, "bg-1").unwrap().unwrap();
        assert_eq!(task.task_id, "bg-1");
        assert!(task.notification_sent);
    }
}
