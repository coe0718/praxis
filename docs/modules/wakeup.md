# Wakeup

> Wake-on-intent — approved interrupt-style session triggering with adaptive scheduling.

## Overview

The wakeup module enables external systems to request an immediate Praxis session by writing a `wake_intent.json` file to the data directory. When the daemon or agent loop polls for work, it consumes the intent and injects its task into the session. Wake intents carry a priority level: **urgent** intents bypass quiet-hours checks, while **normal** intents respect them.

The module also contains the **adaptive scheduler** (`OperatorSchedule`) which learns operator activity patterns over time. After accumulating enough interaction timestamps, it identifies quiet hours automatically and avoids scheduling sessions during low-activity periods.

## Architecture

### Wake-on-Intent

| Type | Description |
|---|---|
| `WakeIntent` | A wake request with reason, source, optional task, priority, and creation timestamp |
| `WakePriority` | `Urgent` (bypasses quiet hours) or `Normal` (respects quiet hours) |

**Flow:**
1. External system (webhook, Telegram, cron) calls `request_wake()` → writes `wake_intent.json`
2. Agent loop or daemon calls `consume_intent()` → reads and deletes the file
3. If the intent is stale (>24h old), it's silently discarded
4. Urgent intents set a flag that bypasses the quiet-hours gate

### Adaptive Scheduler (`schedule.rs`)

| Type | Description |
|---|---|
| `OperatorSchedule` | Hourly interaction histogram (24 bins, UTC hours 0–23) with total sample count |

**Algorithm:**
- `record_activity()` increments the counter for the current UTC hour
- After 20+ samples, `quiet_hours()` identifies hours below 10% of peak activity
- `next_preferred_wake_time()` finds the next non-quiet hour for scheduling
- Persisted in `operator_schedule.json`

## Public API

### Wake Intent

```rust
// Create and write a wake intent
let intent = WakeIntent::new("PR merged", "webhook")
    .with_task("run tests")
    .urgent();
request_wake(&data_dir, &intent)?;

// Consume (read and delete) a pending intent
if let Some(intent) = consume_intent(&data_dir)? {
    println!("Wake from {}: {}", intent.source, intent.reason);
}

// Check if a wake intent is pending (without consuming)
if is_pending(&data_dir) { ... }

// Format for logging
let summary = format_summary(&intent);
```

### Adaptive Schedule

```rust
let mut schedule = OperatorSchedule::load(&paths.operator_schedule_file)?;
schedule.record_activity(now);
schedule.save(&paths.operator_schedule_file)?;

let quiet = schedule.quiet_hours();          // Vec<u32> of quiet UTC hours
let next = schedule.next_preferred_wake_time(now); // DateTime<Utc>
let info = schedule.summary();               // Human-readable string
```

## Configuration

### Wake Intent File Format (`wake_intent.json`)

```json
{
  "reason": "PR merged",
  "source": "webhook",
  "task": "run tests",
  "priority": "urgent",
  "created_at": "2026-04-03T12:00:00Z"
}
```

### Constants

| Constant | Value | Description |
|---|---|---|
| `MAX_AGE_HOURS` | 24 | Intents older than this are discarded as stale |
| `MIN_SAMPLES` | 20 | Minimum interaction samples before quiet-hour detection activates |
| `QUIET_THRESHOLD_PCT` | 0.10 | Hours below this fraction of peak are treated as quiet |

## Usage

### CLI — request a wake

```bash
praxis wake --task "Deploy to production"
praxis wake --task "urgent: Security patch"  # urgent prefix bypasses quiet hours
```

### Webhook integration

```bash
echo '{"reason":"CI green","source":"github","task":"merge PR","priority":"urgent","created_at":"'$date -u +%FT%TZ'"}' \
  > /path/to/data_dir/wake_intent.json
```

### Telegram

```
/wake deploy to staging
```

## Data Files

| File | Purpose |
|---|---|
| `wake_intent.json` | Ephemeral wake trigger — consumed (deleted) after reading |
| `operator_schedule.json` | Persistent hourly activity histogram for adaptive scheduling |

## Dependencies

- External crates: `chrono`, `serde`, `serde_json`

## Source

`src/wakeup/` — `mod.rs`, `schedule.rs`
