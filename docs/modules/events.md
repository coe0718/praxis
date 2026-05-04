# Events

> Structured event emission for the Praxis agent loop.

## Overview

The `events` module provides a lightweight, trait-based event system that allows the agent loop to emit structured events as it runs. Events are simple `(kind, detail)` pairs that are written to an append-only JSONL log, making them suitable for tailing, auditing, or feeding into downstream alerting pipelines.

The module is intentionally minimal — it defines a single `Event` struct, an `EventSink` trait for abstracting where events go, and two implementations: `FileEventSink` (production) and `NoopEventSink` (testing/silent mode). This keeps the event plumbing decoupled from any specific transport or format.

## Architecture

### Types

| Type | Description |
|------|-------------|
| `Event` | A structured event with `kind` (e.g. `"agent:test"`) and `detail` (free-form string). Serialized as JSON. |
| `EventSink` (trait) | Single-method trait: `fn emit(&self, event: &Event) -> Result<()>`. |
| `FileEventSink` | Appends timestamped `EventRecord` entries to a JSONL file. Tracks byte offset for incremental reads. |
| `NoopEventSink` | Discards all events. Used in tests and when event logging is disabled. |
| `EventRecord` | Internal wrapper that adds an `emitted_at` RFC 3339 timestamp to each `Event`. |

### Relationships

`PraxisRuntime` is generic over `E: EventSink`. At runtime the concrete type is `FileEventSink`; in tests it is `NoopEventSink`. The `read_events_since()` function supports incremental tailing by accepting a byte offset and returning `(events, new_offset)`.

## Public API

```rust
// Core types
pub struct Event {
    pub kind: String,
    pub detail: String,
}

pub trait EventSink {
    fn emit(&self, event: &Event) -> Result<()>;
}

pub struct FileEventSink { /* path: PathBuf */ }

impl FileEventSink {
    pub fn new(path: PathBuf) -> Self;
}

// Incremental event reading
pub fn read_events_since(path: &Path, offset: u64) -> Result<(Vec<Event>, u64)>;

// Silent sink
pub struct NoopEventSink;
```

## Configuration

No `praxis.toml` fields or environment variables are specific to this module. The event log path is derived from `PraxisPaths` at runtime. When no event sink is configured, `NoopEventSink` is used by default.

## Usage

Events are emitted internally by the agent loop phases. Operators can tail the JSONL log externally:

```bash
# Watch events in real time
tail -f data_dir/events.jsonl | jq .
```

The `read_events_since()` function is designed for incremental consumption — call it repeatedly with the last returned offset to get only new events.

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `events.jsonl` | JSONL | Append-only event log. Each line is an `EventRecord` with `emitted_at` and `event` fields. |

## Dependencies

- `serde`, `serde_json` — serialization
- `chrono` — timestamps in `FileEventSink`
- `anyhow` — error handling

## Source

`src/events/` — `mod.rs`, `file.rs`
