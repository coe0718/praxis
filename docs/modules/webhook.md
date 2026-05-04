# Webhook

> Direct-delivery webhook processing that bypasses the agent loop for notifications and alerts.

## Overview

The `webhook/` directory contains the direct-delivery extension for Praxis's webhook system. While the main webhook infrastructure (registration, signature verification, HMAC validation) lives in `src/webhooks.rs`, this sub-module handles the "last mile": formatting webhook payloads and delivering them directly to a chat channel, completely bypassing the agent/LLM reasoning loop.

This is designed for notifications, alerts, CI/CD status updates, and other webhook payloads that don't need agent reasoning. Instead of injecting the payload into a `WakeIntent` for the agent to process, the direct-delivery path formats the payload into a readable message and sends it immediately via the appropriate messaging adapter.

The module includes:
- **Route mapping** — A persistent `DeliveryRoutes` store that maps webhook names to target platform + channel ID pairs.
- **Payload formatting** — Smart JSON pretty-printing with summary extraction for common formats (e.g. GitHub push events).
- **Platform dispatch** — Sends to Telegram, Discord, or Slack using the existing messaging adapters.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `DeliveryRoutes` | Persistent map of webhook name → `DeliveryTarget`. Loaded from `webhook_routes.json`. |
| `DeliveryTarget` | A `(platform, channel_id)` pair specifying where to deliver a webhook payload. |
| `DeliveryResult` | Outcome of a delivery attempt: platform, channel, success flag, optional error. |

### Flow

1. A webhook request arrives at `POST /webhook/{name}` (handled in `routes_events`).
2. If the webhook has `direct_delivery = true`, the handler calls `deliver_webhook_to_channel()`.
3. The payload is formatted via `format_webhook_payload()` (pretty-print JSON, extract summaries).
4. The formatted message is sent to the target platform via `deliver_to_platform()`.
5. If no explicit route is configured, falls back to the primary Telegram chat.

## Public API

### `deliver_webhook_to_channel`

```rust
pub fn deliver_webhook_to_channel(
    webhook_name: &str,
    event_type: Option<&str>,
    payload: &str,
    platform: &str,
    channel_id: &str,
) -> DeliveryResult
```

High-level entry point. Formats the payload and delivers it to the specified platform and channel. Returns a `DeliveryResult` indicating success or failure.

### `deliver_to_platform`

```rust
pub fn deliver_to_platform(platform: &str, channel_id: &str, message: &str) -> Result<()>
```

Dispatches to the appropriate messaging adapter. Supports `"telegram"`, `"discord"` (feature-gated), and `"slack"` (feature-gated).

### `resolve_delivery_target`

```rust
pub fn resolve_delivery_target(routes_path: &Path, webhook_name: &str) -> Result<DeliveryTarget>
```

Resolves the delivery target for a webhook name. Falls back to the primary Telegram chat if no explicit route is configured.

### `format_webhook_payload`

```rust
pub fn format_webhook_payload(webhook_name: &str, event_type: Option<&str>, payload: &str) -> String
```

Formats a raw webhook payload into a human-readable chat message. Includes:
- Header with webhook name and event type
- Automatic summary extraction for GitHub push events (commit author + message)
- Pretty-printed JSON (truncated to ~3500 chars)
- Fallback for non-JSON payloads

### `DeliveryRoutes`

```rust
pub fn load(path: &Path) -> Result<Self>
pub fn save(&self, path: &Path) -> Result<()>
pub fn set_route(&mut self, webhook_name: &str, platform: &str, channel_id: &str)
pub fn remove_route(&mut self, webhook_name: &str) -> bool
```

Persistent route map. Loaded from and saved to `webhook_routes.json`.

## Configuration

### Route Map File (`webhook_routes.json`)

```json
{
  "routes": {
    "ci-cd": {
      "platform": "telegram",
      "channel_id": "123456789"
    },
    "deploy-alerts": {
      "platform": "slack",
      "channel_id": "C01234ABCD"
    }
  }
}
```

Webhooks with `direct_delivery = true` (set in `webhooks.json`) will use this route map to determine where to send payloads.

### Feature Flags

| Flag | Required for |
|------|-------------|
| `discord` | Delivering webhooks to Discord channels. |
| `slack` | Delivering webhooks to Slack channels. |

Telegram delivery is always available.

## Usage

### Setting Up Direct Delivery

1. Register a webhook with `direct_delivery = true` in `webhooks.json` (via `src/webhooks.rs` management).
2. Configure a delivery route in `webhook_routes.json`.
3. Inbound requests to `/webhook/{name}` will be forwarded directly to the chat channel.

### Example

```bash
# Register a CI/CD webhook
curl -X POST http://localhost:8080/api/webhooks \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"name": "ci-cd", "description": "CI/CD pipeline", "direct_delivery": true}'

# Configure delivery to a Telegram chat
# (write to webhook_routes.json or use the admin API)

# GitHub webhook → Praxis → Telegram chat (no agent processing)
```

## Data Files

| File | Purpose |
|------|---------|
| `data_dir/webhook_routes.json` | Map of webhook name → (platform, channel_id) for direct delivery. |

## Dependencies

- **`messaging`** — `TelegramBot`, `DiscordClient`, and `SlackClient` for platform dispatch.
- **`webhooks`** — The `Webhook` struct's `direct_delivery` field triggers this code path.

## Source

`src/webhook/`
