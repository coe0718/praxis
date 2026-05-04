# Messaging

> Multi-platform messaging adapters with polling, sending, command routing, activation gating, and sender pairing.

## Overview

The messaging module is Praxis's unified interface for communicating with human operators across Telegram, Discord, and Slack. Rather than treating each platform as a standalone integration, the module provides a shared command vocabulary and routing layer so that `/ask`, `/run`, `/status`, and dozens of other commands work identically regardless of which platform the operator uses.

The module is built around a polling architecture: on each poll cycle, the adapter fetches new messages from the platform API, gates them through per-conversation activation modes and security filters, and publishes accepted messages onto the Praxis message bus. A separate router layer parses commands from message text and dispatches them to CLI handler functions.

Key design principles:
- **One command vocabulary, three platforms.** Telegram, Discord, and Slack share the same command set via `handle_telegram_command`.
- **Activation gating.** Group chats and shared channels can be configured per-conversation to respond only on mentions, thread replies, or always.
- **Sender pairing.** Unknown Telegram chats are not silently dropped; they trigger a one-time code flow where the operator approves the sender from their primary chat.
- **Typing indicators.** Adapters implement the `TypingIndicator` trait to show liveness during active sessions.

## Architecture

### Sub-modules

| Sub-module | Feature Gate | Description |
|-----------|-------------|-------------|
| `telegram` | always | Full Telegram Bot API client with polling, sending, photos, media groups, inline buttons, and typing indicators. |
| `discord` | `discord` | Discord REST client (bot token + webhook). Supports channel messages, action-row buttons, and channel polling. |
| `slack` | `slack` | Slack client (bot token + webhook). Supports `chat.postMessage`, `conversations.history` polling, and URL verification challenges. |
| `router` | always | Command parser and dispatch layer. Shared `TelegramCommand` enum used by all platforms. |
| `activation` | always | Per-conversation activation mode persistence (`activation.json`). |
| `typing` | always | `TypingIndicator` trait + `NoopTypingIndicator` for adapters that don't support presence. |
| `pairing` | always | Sender pairing flow for unknown Telegram chats (`sender_pairing.json`). |
| `context_group` | always | Per-conversation memory and tag pinning (`context_groups.json`). |

### Key Types

| Type | Description |
|------|-------------|
| `TelegramBot` | Telegram Bot API client. Implements `TypingIndicator`. |
| `DiscordClient` | Discord REST + webhook client (feature-gated). |
| `SlackClient` | Slack Web API + webhook client (feature-gated). |
| `TelegramCommand` | Enum of all supported commands (Ask, Run, Status, Queue, Approve, Reject, Goal, Brief, Health, Boundaries, Activation, Memories, Reinforce, Forget, Link, Decisions, ApproveSender, Steer, Fast, Model, Help). |
| `ActivationMode` | `MentionOnly`, `ThreadOnly`, or `AlwaysListening`. Default is `MentionOnly`. |
| `ActivationStore` | Persistent map of conversation ID → activation mode. |
| `MessageGating` | Security settings: `require_mention` and `allowed_users` list. |
| `PairingStore` | Manages pending and approved unknown sender chat IDs. |
| `ContextGroupStore` | Per-conversation pinned memory IDs and tags. |
| `TypingIndicator` | Trait with `begin()` and `end()` methods for liveness signals. |

## Public API

### TelegramBot

```rust
pub fn from_env() -> Result<Self>
pub fn validate_environment() -> Result<()>
pub fn poll_once(&self, state_path, pairing_path, bus, activation, gating, ephemeral_prompts) -> Result<(Vec<TelegramMessage>, Vec<CallbackQuery>)>
pub fn primary_chat_id(&self) -> Option<i64>
pub fn send_message(&self, chat_id: i64, text: &str) -> Result<()>
pub fn send_message_with_buttons(&self, chat_id: i64, text: &str, buttons: &[(&str, &str)]) -> Result<()>
pub fn send_photo(&self, chat_id: i64, image: &str, caption: Option<&str>) -> Result<()>
pub fn send_media_group(&self, chat_id: i64, items: &[(&str, Option<&str>)]) -> Result<()>
pub fn send_typing(&self, chat_id: i64) -> Result<()>
```

### DiscordClient (feature = "discord")

```rust
pub fn from_env() -> Result<Self>
pub fn validate_environment() -> Result<()>
pub fn send_webhook(&self, text: &str, username: Option<&str>) -> Result<()>
pub fn send_message(&self, channel_id: &str, text: &str) -> Result<DiscordMessage>
pub fn send_message_with_buttons(&self, channel_id: &str, text: &str, buttons: &[(&str, &str)]) -> Result<DiscordMessage>
pub fn poll_once(&self, state_path: &Path, allowed_user_ids: &[String]) -> Result<Vec<DiscordPollMessage>>
```

### SlackClient (feature = "slack")

```rust
pub fn from_env() -> Result<Self>
pub fn validate_environment() -> Result<()>
pub fn send_webhook(&self, text: &str) -> Result<()>
pub fn post_message(&self, channel: &str, text: &str) -> Result<String>
pub fn poll_once(&self, state_path: &Path, allowed_user_ids: &[String]) -> Result<Vec<SlackPollMessage>>
```

### Command Router

```rust
pub fn parse_telegram_command(text: &str) -> TelegramCommand
pub fn handle_telegram_command(data_dir: PathBuf, chat_id: &str, text: &str) -> Result<String>
pub fn handle_discord_command(data_dir: PathBuf, channel_id: &str, text: &str) -> Result<String>
pub fn handle_slack_command(data_dir: PathBuf, channel_id: &str, text: &str) -> Result<String>
```

### Activation

```rust
// ActivationStore
pub fn load(path: &Path) -> Result<Self>
pub fn save(&self, path: &Path) -> Result<()>
pub fn get(&self, conversation_id: &str) -> ActivationMode
pub fn set(&mut self, conversation_id: impl Into<String>, mode: ActivationMode)
pub fn remove(&mut self, conversation_id: &str)
pub fn should_respond(&self, conversation_id: &str, is_mention: bool, is_thread_reply: bool) -> bool
```

### Typing Indicator

```rust
pub trait TypingIndicator: Send + Sync {
    fn begin(&self, conversation_id: &str) -> Result<()>;
    fn end(&self, conversation_id: &str) -> Result<()>;
}
```

## Configuration

### Environment Variables

| Variable | Required | Platform | Description |
|----------|----------|----------|-------------|
| `PRAXIS_TELEGRAM_BOT_TOKEN` | Telegram | Telegram | Bot token from @BotFather. |
| `PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS` | Telegram | Telegram | Comma-separated chat IDs. At least one required. |
| `PRAXIS_DISCORD_BOT_TOKEN` | Discord (one of) | Discord | Bot token for Discord REST API. |
| `PRAXIS_DISCORD_WEBHOOK_URL` | Discord (one of) | Discord | Incoming webhook URL for simpler posting. |
| `PRAXIS_DISCORD_CHANNEL_IDS` | Discord polling | Discord | Comma-separated channel IDs for polling. |
| `PRAXIS_DISCORD_ALLOWED_USER_IDS` | Optional | Discord | Comma-separated user IDs to accept messages from. |
| `PRAXIS_DISCORD_PUBLIC_KEY` | Optional | Discord | Application public key for webhook signature verification. |
| `PRAXIS_SLACK_BOT_TOKEN` | Slack (one of) | Slack | Bot OAuth token (`xoxb-...`). |
| `PRAXIS_SLACK_WEBHOOK_URL` | Slack (one of) | Slack | Incoming webhook URL. |
| `PRAXIS_SLACK_CHANNEL_IDS` | Slack polling | Slack | Comma-separated channel IDs for polling. |
| `PRAXIS_SLACK_ALLOWED_USER_IDS` | Optional | Slack | Comma-separated user IDs to accept messages from. |
| `PRAXIS_SLACK_SIGNING_SECRET` | Optional | Slack | Signing secret for webhook HMAC verification. |

### Feature Flags

| Flag | Default | Description |
|------|---------|-------------|
| `discord` | off | Compiles the Discord client. |
| `slack` | off | Compiles the Slack client. |

### Config File (`praxis.toml`)

Security gating for Telegram is configured under `[security]`:

```toml
[security]
require_mention = true
allowed_users = ["123456789"]
```

Per-channel ephemeral prompts can be set under `[messaging]`:

```toml
[messaging.ephemeral_prompts]
"123456789" = "You are in a work context. Keep responses concise."
```

## Usage

### Supported Commands

All platforms share this command set:

| Command | Description |
|---------|-------------|
| `/ask <prompt>` | Quick stateless question |
| `/run <task>` | Full Praxis session |
| `/status` | Session and goal status |
| `/queue` | List pending tool approvals |
| `/approve <id>` | Approve a queued tool request |
| `/reject <id>` | Reject a queued tool request |
| `/goal <description>` | Add a new goal |
| `/brief` | Show recent journal entries |
| `/health` | System health check |
| `/boundaries` | List active boundaries |
| `/boundaries add <rule>` | Add a boundary rule |
| `/activation [mode]` | Get or set activation mode for this conversation |
| `/memories` | List recent memories |
| `/reinforce <id>` | Boost memory importance |
| `/forget <id>` | Delete a memory |
| `/link <from> <type> <to>` | Add a relational memory link |
| `/decisions` | Show last 8 decide-phase receipts |
| `/approve-sender <code>` | Approve an unknown chat by pairing code |
| `/steer <task>` | Redirect running session |
| `/fast` | Toggle fast mode |
| `/model [name]` | Show or set the active model |
| `/model clear` | Reset model override |

### Sender Pairing Flow

1. Unknown chat sends a message → Praxis generates a 6-digit code and sends it to the operator's primary chat.
2. Operator sends `/approve-sender <code>` from the primary chat.
3. The queued message is immediately processed; the chat ID is added to the approved list.

## Data Files

| File | Purpose |
|------|---------|
| `data_dir/telegram_state.json` | Stores `last_update_id` for Telegram polling offset. |
| `data_dir/telegram_state.lock` | Advisory lock file to prevent concurrent polls (auto-removed after 5 min). |
| `data_dir/discord_state.json` | Per-channel last message ID offsets for Discord. |
| `data_dir/slack_state.json` | Per-channel last timestamp offsets for Slack. |
| `data_dir/activation.json` | Per-conversation activation modes. |
| `data_dir/sender_pairing.json` | Pending and approved sender chat IDs. |
| `data_dir/context_groups.json` | Per-conversation pinned memory IDs and tags. |

## Dependencies

- **`bus`** — Publishes `BusEvent` for accepted messages.
- **`cli`** — Dispatches commands to CLI handler functions (`core`, `approvals`).
- **`paths`** — `PraxisPaths` for locating state and config files.
- **`storage`** — `SqliteSessionStore` for memory operations.
- **`boundaries`** — Boundary listing and addition.
- **`lite`** — Fast mode toggle.
- **`wakeup`** — `WakeIntent` for steer and activation.
- **`brief`** — Brief generation.
- **`time`** — System clock.

## Source

`src/messaging/`
