# Heartbeat

> Liveness probe via `heartbeat.json` — a simple JSON file that external monitors watch to confirm Praxis is alive and making progress.

## Overview

The heartbeat module provides a lightweight liveness mechanism. Every time the agent loop completes a phase (or the daemon starts), it writes a `HeartbeatRecord` to `heartbeat.json`. External tools — systemd watchdogs, `praxis status`, or custom monitoring scripts — can check this file to confirm Praxis is running and hasn't stalled.

The heartbeat record includes the current phase, a human-readable detail message, the PID, a UTC timestamp, and the process uptime. A `check_heartbeat()` function reads the record and verifies it isn't older than a configurable threshold, returning an error if the heartbeat is stale.

## Architecture

### Key Types

| Type | Description |
|---|---|
| `HeartbeatRecord` | JSON record with component name, phase, detail, timestamps, PID, and process uptime |

### `HeartbeatRecord` Fields

| Field | Type | Description |
|---|---|---|
| `component` | `String` | Usually `"praxis"` |
| `phase` | `String` | Current phase (e.g., `"orient"`, `"daemon"`, `"sleep"`) |
| `detail` | `String` | Human-readable status message |
| `updated_at` | `String` | RFC 3339 timestamp |
| `updated_at_unix_ms` | `i64` | Unix milliseconds timestamp |
| `pid` | `u32` | Process ID |
| `process_uptime_ms` | `u64` | Milliseconds since process start |

## Public API

### Write Heartbeat

```rust
pub fn write_heartbeat(
    path: &Path,
    component: &str,
    phase: &str,
    detail: &str,
    now: DateTime<Utc>,
) -> Result<()>
```

Writes a new `HeartbeatRecord` to the given path. Called by the agent loop after every phase transition and by the daemon at startup.

### Read Heartbeat

```rust
pub fn read_heartbeat(path: &Path) -> Result<HeartbeatRecord>
```

Reads and deserializes the heartbeat file.

### Check Freshness

```rust
pub fn check_heartbeat<C: Clock>(
    clock: &C,
    path: &Path,
    max_age_seconds: i64,
) -> Result<HeartbeatRecord>
```

Reads the heartbeat and verifies its age is within `max_age_seconds`. Returns an error describing how stale the heartbeat is if it exceeds the threshold. Requires `max_age_seconds > 0`.

## Usage

### Writing (from the agent loop)

```rust
write_heartbeat(
    &paths.heartbeat_file,
    "praxis",
    "orient",
    "Loading identity, goals, and context.",
    now,
)?;
```

### Checking (from CLI or monitoring)

```rust
let clock = SystemClock::from_env()?;
match check_heartbeat(&clock, &paths.heartbeat_file, 300) {
    Ok(record) => println!("Agent alive: phase={}", record.phase),
    Err(e) => eprintln!("Heartbeat stale or missing: {e}"),
}
```

### CLI

```bash
praxis heartbeat status    # show current heartbeat record
praxis heartbeat check     # verify freshness with default threshold
```

## Data Files

| File | Purpose |
|---|---|
| `heartbeat.json` | JSON heartbeat record, overwritten on every phase transition |

## Dependencies

- **time** — `Clock` trait for testable time access
- External crates: `chrono`, `serde`, `serde_json`

## Source

`src/heartbeat.rs`
