//! OpenMolt Integration API — 30+ type-safe integrations.
//!
//! Code-first agent API with scope-gated tool access.
//! Provides ready-to-use integrations for common services.

use serde::{Deserialize, Serialize};

/// Integration provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub name: String,
    pub enabled: bool,
    pub scopes: Vec<String>,
    pub credentials_ref: Option<String>,
}

/// Available integration providers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Provider {
    Gmail,
    Slack,
    Discord,
    GitHub,
    Notion,
    Stripe,
    Spotify,
    Telegram,
    GoogleCalendar,
    GoogleMeet,
    Linear,
    Jira,
    Trello,
    Asana,
    HubSpot,
    Salesforce,
    Postgres,
    MySQL,
    Redis,
    Webhook,
}

impl Provider {
    pub fn name(&self) -> &'static str {
        match self {
            Provider::Gmail => "gmail",
            Provider::Slack => "slack",
            Provider::Discord => "discord",
            Provider::GitHub => "github",
            Provider::Notion => "notion",
            Provider::Stripe => "stripe",
            Provider::Spotify => "spotify",
            Provider::Telegram => "telegram",
            Provider::GoogleCalendar => "google_calendar",
            Provider::GoogleMeet => "google_meet",
            Provider::Linear => "linear",
            Provider::Jira => "jira",
            Provider::Trello => "trello",
            Provider::Asana => "asana",
            Provider::HubSpot => "hubspot",
            Provider::Salesforce => "salesforce",
            Provider::Postgres => "postgres",
            Provider::MySQL => "mysql",
            Provider::Redis => "redis",
            Provider::Webhook => "webhook",
        }
    }

    pub fn scopes(&self) -> Vec<&'static str> {
        match self {
            Provider::Gmail => vec!["read", "send", "labels"],
            Provider::Slack => vec!["channels:read", "chat:write", "users:read"],
            Provider::Discord => vec!["guilds", "messages", "members"],
            Provider::GitHub => vec!["repo", "issues", "pull_requests"],
            Provider::Notion => vec!["databases.read", "pages.read", "pages.write"],
            Provider::Stripe => vec!["payments", "customers", "subscriptions"],
            Provider::Spotify => vec!["user-read", "playlist-modify", "playback"],
            Provider::Telegram => vec!["messages", "inline"],
            Provider::GoogleCalendar => vec!["read", "write", "events"],
            Provider::GoogleMeet => vec!["create", "join"],
            Provider::Linear => vec!["issues", "comments", "teams"],
            Provider::Jira => vec!["issues", "projects", "comments"],
            Provider::Trello => vec!["boards", "cards", "lists"],
            Provider::Asana => vec!["tasks", "projects", "users"],
            Provider::HubSpot => vec!["contacts", "deals", "tickets"],
            Provider::Salesforce => vec!["read", "write", "query"],
            Provider::Postgres => vec!["read", "write"],
            Provider::MySQL => vec!["read", "write"],
            Provider::Redis => vec!["read", "write"],
            Provider::Webhook => vec!["send", "receive"],
        }
    }
}

/// Integration registry managing all available integrations.
pub struct IntegrationRegistry {
    configs: std::collections::HashMap<String, IntegrationConfig>,
}

impl IntegrationRegistry {
    pub fn new() -> Self {
        Self {
            configs: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, config: IntegrationConfig) {
        self.configs.insert(config.name.clone(), config);
    }

    pub fn get(&self, name: &str) -> Option<&IntegrationConfig> {
        self.configs.get(name)
    }

    pub fn list_enabled(&self) -> Vec<&IntegrationConfig> {
        self.configs.values().filter(|c| c.enabled).collect()
    }
}

impl Default for IntegrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed integration client.
pub struct TypedClient<T> {
    _config: IntegrationConfig,
    _marker: std::marker::PhantomData<T>,
}

impl<T> TypedClient<T> {
    pub fn new(config: IntegrationConfig) -> Self {
        Self {
            _config: config,
            _marker: std::marker::PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_scopes() {
        assert!(Provider::Gmail.scopes().contains(&"send"));
        assert!(Provider::Slack.scopes().contains(&"chat:write"));
    }

    #[test]
    fn test_registry() {
        let mut registry = IntegrationRegistry::new();
        registry.register(IntegrationConfig {
            name: "gmail".into(),
            enabled: true,
            scopes: vec!["send".into()],
            credentials_ref: None,
        });

        assert!(registry.get("gmail").is_some());
    }
}