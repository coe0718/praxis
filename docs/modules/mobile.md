# Mobile

> Push notifications and mobile integration for urgent alerts, session summaries, and proactive updates.

## Overview

The mobile module enables Praxis to send push notifications to the operator's mobile device. It provides a `MobileAgent` that wraps notification delivery with configurable device tokens, priority levels, and notification categories.

Notifications are sent via the `notify()` method and are currently logged rather than delivered to an actual push service (APNs/FCM). The module is designed as a scaffold: replace the log call with a real push notification SDK call for production use.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `MobileNotification` | A notification payload with title, body, priority, optional category, and custom data. |
| `NotificationPriority` | Enum: `Low`, `Normal` (default), `High`. |
| `MobileAgent` | Manages device token, enabled state, and notification dispatch. |

### Relationships

`MobileAgent` owns an optional device token and an enable flag. It creates `MobileNotification` instances internally via convenience methods (`urgent`, `session_summary`) and dispatches them through `notify()`.

## Public API

### `MobileNotification`

```rust
pub struct MobileNotification {
    pub title: String,
    pub body: String,
    pub priority: NotificationPriority,
    pub category: Option<String>,
    pub data: HashMap<String, String>,
}
```

The full notification payload sent to the mobile device.

### `NotificationPriority`

```rust
pub enum NotificationPriority {
    Low,
    Normal,
    High,
}
```

Controls the urgency level. `Normal` is the default via `#[default]`.

### `MobileAgent`

```rust
impl MobileAgent {
    pub fn new() -> Self
    pub fn from_file(path: &Path) -> Self
    pub fn save(&self, path: &Path) -> Result<()>
    pub fn notify(&self, notification: &MobileNotification) -> Result<()>
    pub fn urgent(&self, title: &str, body: &str) -> Result<()>
    pub fn session_summary(&self, outcome: &str, summary: &str) -> Result<()>
}
```

- **`new`** — Creates a `MobileAgent` with `enabled: false`, no device token, and `auto_notify: false`.
- **`from_file`** — Loads agent configuration from a JSON file. Returns defaults on read/parse errors.
- **`save`** — Saves the agent configuration to a JSON file.
- **`notify`** — Sends a notification. Silently returns if `enabled` is false or no device token is configured. Logs the notification details with only the first 8 characters of the token for privacy.
- **`urgent`** — Convenience method that sends a `High` priority notification with category `"alert"`.
- **`session_summary`** — Convenience method that sends a `Normal` priority notification with category `"session"`.

## Configuration

Serialized as JSON, not TOML:

```json
{
  "device_token": "apns-or-fcm-token",
  "enabled": true,
  "auto_notify": true
}
```

## Usage

```rust
use praxis::mobile::{MobileAgent, MobileNotification, NotificationPriority};

let agent = MobileAgent::new();
agent.save("/path/to/mobile_config.json")?;

// Send an urgent alert
agent.urgent("System Alert", "Disk space critically low")?;

// Send a session summary
agent.session_summary("completed", "Processed 3 tasks successfully")?;
```

## Data Files

| File | Purpose |
|------|---------|
| `mobile_config.json` | JSON file with device token, enabled flag, and auto_notify setting. |

## Dependencies

- **`serde` / `serde_json`** — Serialization of agent config and notifications.

## Source

`src/mobile.rs`