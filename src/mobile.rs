//! Mobile App Framework — Push notifications and mobile integration.
//!
//! Enables Praxis to send push notifications to operator's mobile device
//! for urgent alerts, session summaries, and proactive updates.

use serde::{Deserialize, Serialize};

/// Mobile notification message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileNotification {
    pub title: String,
    pub body: String,
    pub priority: NotificationPriority,
    pub category: Option<String>,
    pub data: std::collections::HashMap<String, String>,
}

/// Notification priority level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum NotificationPriority {
    Low,
    #[default]
    Normal,
    High,
}

/// Mobile agent for sending push notifications.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MobileAgent {
    pub device_token: Option<String>,
    pub enabled: bool,
    pub auto_notify: bool,
}

impl MobileAgent {
    /// Create new mobile agent.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from configuration.
    pub fn from_file(path: &std::path::Path) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    /// Save configuration.
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Send a notification to the operator's mobile device.
    pub fn notify(&self, notification: &MobileNotification) -> anyhow::Result<()> {
        if !self.enabled {
            log::debug!("mobile: notifications disabled");
            return Ok(());
        }

        let device_token = match &self.device_token {
            Some(t) => t,
            None => {
                log::warn!("mobile: no device token configured");
                return Ok(());
            }
        };

        // Would send via APNs/FCM in production
        log::info!(
            "mobile: [{}] {} - {} (token: {})",
            notification.priority.as_str(),
            notification.title,
            notification.body,
            &device_token[..8] // Log only first 8 chars for privacy
        );

        Ok(())
    }

    /// Send urgent alert.
    pub fn urgent(&self, title: &str, body: &str) -> anyhow::Result<()> {
        let notification = MobileNotification {
            title: title.to_string(),
            body: body.to_string(),
            priority: NotificationPriority::High,
            category: Some("alert".to_string()),
            data: std::collections::HashMap::new(),
        };
        self.notify(&notification)
    }

    /// Send session summary.
    pub fn session_summary(&self, outcome: &str, summary: &str) -> anyhow::Result<()> {
        let notification = MobileNotification {
            title: format!("Session: {}", outcome),
            body: summary.to_string(),
            priority: NotificationPriority::Normal,
            category: Some("session".to_string()),
            data: std::collections::HashMap::new(),
        };
        self.notify(&notification)
    }
}

impl NotificationPriority {
    fn as_str(&self) -> &'static str {
        match self {
            NotificationPriority::Low => "low",
            NotificationPriority::Normal => "normal",
            NotificationPriority::High => "high",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_priority_default() {
        assert!(matches!(NotificationPriority::default(), NotificationPriority::Normal));
    }

    #[test]
    fn test_mobile_agent_default() {
        let agent = MobileAgent::new();
        assert!(!agent.enabled);
        assert!(agent.device_token.is_none());
    }
}