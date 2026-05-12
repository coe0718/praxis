# Circuit Breaker

> Prevent cascading failures by stopping requests to failing services — implements the circuit breaker pattern: closed (normal), open (failing), half-open (testing recovery).

## Overview

The circuit breaker module provides per-service failure detection and isolation. Each `CircuitBreaker` tracks failure and success counts and transitions between three states:

- **Closed** — Normal operation; requests pass through.
- **Open** — Failures exceeded threshold; requests fail-fast without calling the downstream service.
- **Half-Open** — After a configurable timeout, allows a limited number of test requests; if they succeed the circuit closes, otherwise it re-opens.

State is managed behind a single `Mutex` to prevent deadlocks and enable shared state across concurrent accesses (C7/C8 fixes). The `CircuitBreakerRegistry` provides named circuit breakers with shared `Arc` ownership so all callers see the same state.

**Current status:** Fully implemented. Used as a building block for tool execution and external API resilience.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `CircuitState` | Enum: `Closed`, `Open`, `HalfOpen`. |
| `CircuitBreakerConfig` | Configuration: failure threshold (default 5), success threshold (default 2), timeout (default 30s). |
| `CircuitBreaker` | Per-service circuit breaker instance with thread-safe state. |
| `CircuitBreakerRegistry` | Named registry of shared circuit breakers behind `Arc`. |

### State Machine

```
Closed ──(failures >= threshold)──→ Open ──(timeout elapsed)──→ HalfOpen
  ↑                                                               │
  └──────────────(successes >= threshold)─────────────────────────┘
  Any failure in HalfOpen → Open
```

## Public API

### `CircuitBreaker`

```rust
impl CircuitBreaker {
    pub fn new(name: &str) -> Self
    pub fn with_config(name: &str, config: CircuitBreakerConfig) -> Self
    pub fn state(&self) -> CircuitState
    pub fn is_available(&self) -> bool
    pub fn record_success(&self)
    pub fn record_failure(&self)
    pub fn reset(&self)
    pub fn name(&self) -> &str
    pub fn failures(&self) -> u64
}
```

- **`new`** — Creates with default config (threshold 5, success 2, timeout 30s).
- **`with_config`** — Creates with custom config.
- **`state`** — Returns current state; auto-transitions Open→HalfOpen if timeout elapsed.
- **`is_available`** — Returns `true` if Closed or HalfOpen.
- **`record_success`** — Increments success count; in HalfOpen, transitions to Closed if threshold met.
- **`record_failure`** — Increments failure count; in Closed, transitions to Open if threshold met; in HalfOpen, immediately re-opens.
- **`reset`** — Resets state to Closed with zero counters.
- **`failures`** — Returns current failure count.

### `CircuitBreakerRegistry`

```rust
impl CircuitBreakerRegistry {
    pub fn new() -> Self
    pub fn get(&self, name: &str) -> Arc<CircuitBreaker>
    pub fn states(&self) -> Vec<(String, CircuitState, u64)>
}
```

- **`get`** — Returns an `Arc` to an existing or new circuit breaker; callers share state.
- **`states`** — Returns all circuit states with names and failure counts.

### `CircuitState`

```rust
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}
```

### `CircuitBreakerConfig`

```rust
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,    // default: 5
    pub success_threshold: u32,    // default: 2
    pub timeout: Duration,         // default: 30s
}
```

## Configuration

No `praxis.toml` fields. Configured programmatically:

```rust
use praxis::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerRegistry};
use std::time::Duration;

let config = CircuitBreakerConfig {
    failure_threshold: 10,
    success_threshold: 3,
    timeout: Duration::from_secs(60),
};
let cb = CircuitBreaker::with_config("my_service", config);

// Use via registry for shared state
let reg = CircuitBreakerRegistry::new();
let shared_cb = reg.get("api_service");
shared_cb.record_failure();
```

## Dependencies

- **`std::sync::{Arc, Mutex}`** — Thread-safe state with shared ownership.
- **`std::time::{Duration, Instant}`** — Time-based state transitions.

## Status

- ✅ Three-state circuit breaker (Closed/Open/HalfOpen)
- ✅ Auto-transition Open→HalfOpen after timeout
- ✅ Configurable failure/success thresholds
- ✅ Thread-safe with single Mutex (C8 fix)
- ✅ Shared state via Arc in registry
- ✅ Comprehensive test coverage

## Source

`src/circuit_breaker.rs`