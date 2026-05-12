# OpenMolt

> Code-first agent API with 20 type-safe integration providers and scope-gated tool access.

## Overview

The OpenMolt module provides an integration framework with a registry of 20 providers (Gmail, Slack, Discord, GitHub, Notion, Stripe, Spotify, Telegram, Google Calendar, Google Meet, Linear, Jira, Trello, Asana, HubSpot, Salesforce, Postgres, MySQL, Redis, Webhook), each with a set of predefined OAuth-style scopes.

Integrations are registered via `IntegrationRegistry`, configured with `IntegrationConfig` (name, enabled flag, scope list, optional credentials reference), and can be wrapped in a `TypedClient<T>` for type-safe access. The `register_integrations()` function wires the provider list into the tool registry at orient time.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `Provider` | Enum of 20 supported integration services. |
| `IntegrationConfig` | Configuration for a single integration (name, enabled, scopes, credentials_ref). |
| `IntegrationRegistry` | HashMap-backed registry of integration configs. |
| `TypedClient<T>` | Generic typed client wrapper around an `IntegrationConfig`. |
| `PROVIDER_LIST` | Static constant slice of all `Provider` variants. |

### Relationships

`IntegrationRegistry` holds a `HashMap<String, IntegrationConfig>`. `TypedClient<T>` wraps a single config with a `PhantomData<T>` marker for type safety. `register_integrations()` is a free function that logs and returns a summary string.

## Public API

### `Provider`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Provider {
    Gmail, Slack, Discord, GitHub, Notion,
    Stripe, Spotify, Telegram,
    GoogleCalendar, GoogleMeet,
    Linear, Jira, Trello, Asana,
    HubSpot, Salesforce,
    Postgres, MySQL, Redis,
    Webhook,
}

impl Provider {
    pub fn name(&self) -> &'static str
    pub fn scopes(&self) -> Vec<&'static str>
}
```

- **`name`** — Canonical string name (e.g., `"gmail"`, `"slack"`, `"github"`).
- **`scopes`** — Default OAuth-style scopes for the provider:

| Provider | Scopes |
|----------|--------|
| Gmail | `read`, `send`, `labels` |
| Slack | `channels:read`, `chat:write`, `users:read` |
| Discord | `guilds`, `messages`, `members` |
| GitHub | `repo`, `issues`, `pull_requests` |
| Notion | `databases.read`, `pages.read`, `pages.write` |
| Stripe | `payments`, `customers`, `subscriptions` |
| Spotify | `user-read`, `playlist-modify`, `playback` |
| Telegram | `messages`, `inline` |
| GoogleCalendar | `read`, `write`, `events` |
| GoogleMeet | `create`, `join` |
| Linear | `issues`, `comments`, `teams` |
| Jira | `issues`, `projects`, `comments` |
| Trello | `boards`, `cards`, `lists` |
| Asana | `tasks`, `projects`, `users` |
| HubSpot | `contacts`, `deals`, `tickets` |
| Salesforce | `read`, `write`, `query` |
| Postgres | `read`, `write` |
| MySQL | `read`, `write` |
| Redis | `read`, `write` |
| Webhook | `send`, `receive` |

### `PROVIDER_LIST`

```rust
pub const PROVIDER_LIST: &[Provider] = &[
    Provider::Gmail, Provider::Slack, Provider::Discord, Provider::GitHub,
    Provider::Notion, Provider::Stripe, Provider::Spotify, Provider::Telegram,
    Provider::GoogleCalendar, Provider::GoogleMeet,
    Provider::Linear, Provider::Jira, Provider::Trello, Provider::Asana,
    Provider::HubSpot, Provider::Salesforce,
    Provider::Postgres, Provider::MySQL, Provider::Redis, Provider::Webhook,
];
```

### `IntegrationConfig`

```rust
pub struct IntegrationConfig {
    pub name: String,
    pub enabled: bool,
    pub scopes: Vec<String>,
    pub credentials_ref: Option<String>,
}
```

### `IntegrationRegistry`

```rust
impl IntegrationRegistry {
    pub fn new() -> Self
    pub fn register(&mut self, config: IntegrationConfig)
    pub fn get(&self, name: &str) -> Option<&IntegrationConfig>
    pub fn list_enabled(&self) -> Vec<&IntegrationConfig>
}
```

- **`new`** — Creates an empty registry.
- **`register`** — Adds or replaces an integration config by name.
- **`get`** — Looks up an integration by name.
- **`list_enabled`** — Returns all integrations with `enabled: true`.

### `TypedClient<T>`

```rust
pub struct TypedClient<T> {
    _config: IntegrationConfig,
    _marker: PhantomData<T>,
}

impl<T> TypedClient<T> {
    pub fn new(config: IntegrationConfig) -> Self
}
```

A type-safe wrapper that pairs an `IntegrationConfig` with a marker type. The concrete `T` is resolved by the consumer (e.g., `TypedClient<GmailProvider>`).

### Free Functions

```rust
pub fn register_integrations() -> String
```

Logs the list of available providers and returns a summary string like `"20 integrations available: gmail, slack, discord, ..."`. Intended for use during the orient phase to make the agent aware of available integrations.

## Configuration

```toml
[openmolt]
[[openmolt.integrations]]
name = "gmail"
enabled = true
scopes = ["read", "send"]
credentials_ref = "gmail-oauth-credentials"

[[openmolt.integrations]]
name = "github"
enabled = true
scopes = ["repo", "issues"]
credentials_ref = "github-token"
```

## Usage

```rust
use praxis::openmolt::{
    IntegrationRegistry, IntegrationConfig, TypedClient, register_integrations,
};

let summary = register_integrations();
println!("{summary}");

let mut registry = IntegrationRegistry::new();
registry.register(IntegrationConfig {
    name: "gmail".into(),
    enabled: true,
    scopes: vec!["send".into(), "read".into()],
    credentials_ref: Some("gmail-creds".into()),
});

let enabled = registry.list_enabled();

// Typed client example
struct GmailProvider;
let client: TypedClient<GmailProvider> = TypedClient::new(
    IntegrationConfig {
        name: "gmail".into(),
        enabled: true,
        scopes: vec![],
        credentials_ref: None,
    },
);
```

## Data Files

None. Configurations are managed in-memory or loaded from `praxis.toml`.

## Dependencies

- **`serde`** — Serialization for `Provider`, `IntegrationConfig`.

## Source

`src/openmolt.rs`