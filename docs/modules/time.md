# Time

> The `Clock` trait and time utilities — dependency-injected time for deterministic testing.

## Overview

The time module provides a `Clock` trait that abstracts time access, along with utility functions for timezone parsing, quiet-hours detection, and clock-time parsing. By depending on the `Clock` trait rather than calling `Utc::now()` directly, Praxis makes its time-dependent logic fully testable — tests inject a `FixedClock` that returns a predetermined timestamp.

Two clock implementations are provided: `SystemClock` (production, reads real time but can be overridden via `PRAXIS_FIXED_NOW` for debugging) and `FixedClock` (testing, always returns a fixed timestamp).

## Architecture

### Key Types

| Type | Description |
|---|---|
| `Clock` (trait) | Single method: `fn now_utc(&self) -> DateTime<Utc>` |
| `SystemClock` | Production clock. Reads real UTC time. Optionally pinned to a fixed timestamp via `PRAXIS_FIXED_NOW` env var. |
| `FixedClock` | Test clock. Always returns a fixed `DateTime<Utc>`. |

### Utility Functions

| Function | Description |
|---|---|
| `parse_timezone(name)` | Parse an IANA timezone string (e.g., `"America/New_York"`) into a `chrono_tz::Tz` |
| `parse_clock_time(value)` | Parse `"HH:MM"` into a `NaiveTime` |
| `is_quiet_hours(now_utc, timezone, start, end)` | Returns `true` if the current local time falls within the quiet-hours window. Handles overnight windows (e.g., 23:00–07:00). |

## Public API

### Clock Trait

```rust
pub trait Clock {
    fn now_utc(&self) -> DateTime<Utc>;
}
```

### SystemClock

```rust
impl SystemClock {
    pub fn from_env() -> Result<Self>
}
```

Reads `PRAXIS_FIXED_NOW` environment variable. If set (RFC 3339 format), all `now_utc()` calls return that fixed time. Otherwise, returns real wall-clock time.

### FixedClock

```rust
impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self
}
```

Always returns the given timestamp. Used in unit tests.

### Quiet Hours

```rust
pub fn is_quiet_hours(
    now_utc: DateTime<Utc>,
    timezone: &str,         // IANA timezone
    quiet_hours_start: &str, // "HH:MM"
    quiet_hours_end: &str,   // "HH:MM"
) -> Result<bool>
```

Converts `now_utc` to local time using the given timezone, then checks whether it falls between start and end. When start > end (e.g., 23:00–07:00), the check spans midnight. When start == end, quiet hours are disabled.

## Configuration

### Environment Variables

| Variable | Format | Description |
|---|---|---|
| `PRAXIS_FIXED_NOW` | RFC 3339 (e.g., `2026-04-03T12:00:00Z`) | Pin `SystemClock` to a fixed timestamp for debugging or replay |

### `praxis.toml` — `[instance]` section

| Field | Description |
|---|---|
| `timezone` | IANA timezone for quiet-hours calculation (e.g., `"America/New_York"`) |

### `praxis.toml` — `[runtime]` section

| Field | Description |
|---|---|
| `quiet_hours_start` | Start of quiet hours in local time (`"HH:MM"`) |
| `quiet_hours_end` | End of quiet hours in local time (`"HH:MM"`) |

## Usage

```rust
let clock = SystemClock::from_env()?;
let now = clock.now_utc();

if is_quiet_hours(now, "UTC", "23:00", "07:00")? {
    println!("Quiet hours active");
}
```

In tests:

```rust
let clock = FixedClock::new(specific_datetime);
assert!(is_quiet_hours(clock.now_utc(), "UTC", "23:00", "07:00").unwrap());
```

## Dependencies

- External crates: `chrono`, `chrono_tz`

## Source

`src/time.rs`
