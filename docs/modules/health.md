# Health Monitor

> Periodic health checks with alerting — registers health checks for subsystems, runs them periodically, and provides an overall health endpoint for load balancer probes.

## Overview

The health monitor module provides a framework for registering and running periodic health checks on Praxis subsystems. Each check is a closure returning a `HealthStatus` variant: `Healthy`, `Degraded`, `Unhealthy`, or `Unknown`. Checks are run on demand via `check_all()`, which returns detailed `HealthCheckResult` structs with timing and optional details.

The monitor tracks consecutive failures per check and logs warnings when they exceed a configurable threshold. An `overall_health()` method aggregates all checks into a single status (unhealthy takes precedence over degraded, which takes precedence over healthy). Uptime tracking is also provided.

**Current status:** Fully implemented. Used internally and exposed for load balancer integration.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `HealthStatus` | Enum: `Healthy`, `Degraded { reason }`, `Unhealthy { reason }`, `Unknown`. |
| `HealthCheckResult` | A single check result: name, status, timestamp, duration, optional details. |
| `HealthMonitor` | Monitor holding registered checks (behind `Mutex`), start time, and aggregation logic. |
| `HealthCheckFn` | `Box<dyn Fn() -> HealthStatus + Send + Sync>` — the check function signature. |

## Public API

### `HealthStatus`

```rust
pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
    Unknown,
}
```

### `HealthCheckResult`

```rust
pub struct HealthCheckResult {
    pub name: String,
    pub status: HealthStatus,
    pub checked_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub details: Option<serde_json::Value>,
}
```

### `HealthMonitor`

```rust
impl HealthMonitor {
    pub fn new() -> Self
    pub fn register(&self, name: &str, alert_threshold: u32, check_fn: HealthCheckFn)
    pub fn check_all(&self) -> Vec<HealthCheckResult>
    pub fn overall_health(&self) -> HealthStatus
    pub fn last_results(&self) -> Vec<HealthCheckResult>
    pub fn uptime_secs(&self) -> u64
    pub fn check_count(&self) -> usize
}
```

- **`register`** — Register a health check with a name, alert threshold (consecutive failures before logging a warning), and check function.
- **`check_all`** — Runs all registered checks, records results, updates consecutive failure counters, logs warnings on threshold breach.
- **`overall_health`** — Aggregates last results: any `Unhealthy` → `Unhealthy`; any `Degraded` → `Degraded`; empty or all healthy → `Healthy`.
- **`last_results`** — Returns cached results without re-running checks.
- **`uptime_secs`** — Seconds since monitor creation.

## Configuration

Configured in `praxis.toml`:

```toml
[health]
check_interval_secs = 60
alert_on_failure = true
```

### Programmatic

```rust
use praxis::health::{HealthMonitor, HealthStatus};
use std::time::Duration;

let monitor = HealthMonitor::new();

// Register a check for the database connection
monitor.register("database", 3, Box::new(|| {
    // Check database connectivity
    // if connected { HealthStatus::Healthy }
    // else { HealthStatus::Unhealthy { reason: "cannot connect".into() } }
    HealthStatus::Healthy
}));

// Run all checks
let results = monitor.check_all();

// Get overall health
match monitor.overall_health() {
    HealthStatus::Healthy => println!("All systems go"),
    HealthStatus::Degraded { reason } => println!("Degraded: {}", reason),
    HealthStatus::Unhealthy { reason } => println!("UNHEALTHY: {}", reason),
    HealthStatus::Unknown => println!("Status unknown"),
}

// For load balancer health endpoint
// GET /health → JSON serialization of monitor.last_results()
```

## Dependencies

- **`std::sync::Mutex`** — Thread-safe check registration and results.
- **`std::time::Instant`** — Check timing and uptime measurement.
- **`chrono`** — Timestamps for check results.
- **`serde` / `serde_json`** — Serialization for results and optional details.

## Status

- ✅ Four-state health enum (Healthy/Degraded/Unhealthy/Unknown)
- ✅ Closure-based check registration
- ✅ Consecutive failure tracking with alert threshold
- ✅ Overall health aggregation
- ✅ Cached last results (no re-run needed for probes)
- ✅ Uptime tracking
- ✅ Comprehensive test coverage

## Source

`src/health.rs`