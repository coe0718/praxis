# Bus

> Central message bus — the transport spine for all adapter-sourced inbound events.

## Overview

The bus module provides a normalized transport layer for inbound events from all messaging adapters (Telegram, Discord, webhooks, voice). Instead of each adapter calling into the agent loop directly, adapters publish `BusEvent` instances onto the bus. The daemon loop, approval queue, dashboard, and hook system subscribe to the bus by draining events.

The bus is deliberately separate from the heartbeat module. The heartbeat (`heartbeat.json`) records process health; the bus (`bus.jsonl`) records inbound user intent. They serve different monitoring purposes.

Two implementations are provided: `FileBus` (production, backed by an append-only JSONL file) and `NullBus` (testing, a no-op sink).

## Architecture

### Key Types

| Type | Description |
|---|---|
| `BusEvent` | A normalized inbound event with kind, source, conversation ID, sender ID, payload, and timestamp |
| `MessageBus` (trait) | Publish/peek/drain interface for the bus |
| `FileBus` | JSONL-backed implementation — append on publish, read-and-clear on drain |
| `NullBus` | No-op implementation for tests and contexts that don't use the bus |

### `BusEvent` Fields

| Field | Type | Description |
|---|---|---|
| `kind` | `String` | Event kind: `"message"`, `"voice"`, `"file"`, `"command"` |
| `source` | `String` | Adapter: `"telegram"`, `"discord"`, `"webhook"` |
| `conversation_id` | `String` | Conversation or channel identifier (opaque to the bus) |
| `sender_id` | `String` | Sender identifier (opaque to the bus) |
| `payload` | `String` | Normalized text or content |
| `published_at` | `DateTime<Utc>` | Timestamp |

### FileBus Behavior

- **`publish()`**: Appends a single JSON line to `bus.jsonl`. Creates the file and parent directories if needed.
- **`drain()`**: Reads all events, then truncates the file to empty. Returns events in order.
- **`peek()`**: Reads all events without consuming them (no file modification).

Malformed lines are silently skipped during read (resilient to partial writes or corruption).

## Public API

### MessageBus Trait

```rust
pub trait MessageBus: Send + Sync {
    fn publish(&self, event: &BusEvent) -> Result<()>;
    fn drain(&self) -> Result<Vec<BusEvent>>;
    fn peek(&self) -> Result<Vec<BusEvent>>;
}
```

### Creating Events

```rust
let event = BusEvent::new(
    "message",           // kind
    "telegram",          // source
    "chat:123456",       // conversation_id
    "user:789",          // sender_id
    "Hello, Praxis!"     // payload
);
```

### FileBus

```rust
let bus = FileBus::new(paths.bus_file.clone());
bus.publish(&event)?;
let events = bus.drain()?;   // Returns events and clears file
```

### NullBus

```rust
let bus = NullBus;
bus.publish(&event)?;        // No-op
assert!(bus.drain()?.is_empty());
```

## Usage

### From an Adapter (Telegram)

```rust
let event = BusEvent::new("message", "telegram", chat_id, user_id, text);
bus.publish(&event)?;
```

### From the Daemon Loop

```rust
let bus = FileBus::new(paths.bus_file.clone());
let events = bus.drain()?;
for event in &events {
    // Process inbound message
}
```

The daemon uses `BusWatcher` (in `daemon.rs`) which detects file growth rather than draining — the actual drain happens during session setup.

## Data Files

| File | Purpose |
|---|---|
| `bus.jsonl` | Append-only JSONL file. One `BusEvent` per line. Cleared on drain. |

## Dependencies

- External crates: `chrono`, `serde`, `serde_json`

## Source

`src/bus/` — `mod.rs`
