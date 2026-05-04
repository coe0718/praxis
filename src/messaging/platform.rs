//! Platform trait — abstract interface for messaging platforms.  (#28)
//!
//! Each messaging platform (Telegram, Discord, Slack, etc.) implements the
//! `Platform` trait so the gateway can treat them uniformly.  New platforms
//! can be added as plugins by implementing this trait and registering in
//! the `PlatformRegistry`.

use std::fmt;

use anyhow::Result;

/// Abstract messaging platform — the gateway talks to platforms through this
/// trait so new platforms (Teams, Matrix, IRC, etc.) can be added without
/// modifying the gateway core.  (#28)
pub trait Platform: fmt::Debug + Send + Sync {
    /// Human-readable platform name (e.g. "telegram", "discord", "slack").
    fn name(&self) -> &str;

    /// Check if the platform is connected and healthy.
    fn is_connected(&self) -> bool;

    /// Send a text message to a target (chat_id, channel_id, etc.).
    fn send_message(&self, target: &str, text: &str) -> Result<()>;

    /// Send a file attachment to a target.
    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()>;

    /// Send a typing indicator to a target.
    fn send_typing(&self, target: &str) -> Result<()>;

    /// Return the number of pending unread messages (best-effort).
    fn pending_count(&self) -> usize {
        0
    }
}

/// Registry of active platforms.  Gateway iterates platforms to dispatch
/// outbound messages and poll for inbound messages.  (#28)
pub struct PlatformRegistry {
    platforms: Vec<Box<dyn Platform>>,
}

impl PlatformRegistry {
    pub fn new() -> Self {
        Self { platforms: Vec::new() }
    }

    /// Register a platform instance.
    pub fn register(&mut self, platform: Box<dyn Platform>) {
        log::info!("registered platform: {}", platform.name());
        self.platforms.push(platform);
    }

    /// Get a platform by name.
    pub fn get(&self, name: &str) -> Option<&dyn Platform> {
        self.platforms.iter().find(|p| p.name() == name).map(|p| p.as_ref())
    }

    /// List all registered platform names.
    pub fn list(&self) -> Vec<&str> {
        self.platforms.iter().map(|p| p.name()).collect()
    }

    /// Broadcast a message to all connected platforms.
    pub fn broadcast(&self, text: &str) -> Vec<Result<()>> {
        self.platforms.iter().map(|p| p.send_message("broadcast", text)).collect()
    }

    /// Number of registered platforms.
    pub fn len(&self) -> usize {
        self.platforms.len()
    }

    /// Whether any platforms are registered.
    pub fn is_empty(&self) -> bool {
        self.platforms.is_empty()
    }
}

impl Default for PlatformRegistry {
    fn default() -> Self {
        Self::new()
    }
}
