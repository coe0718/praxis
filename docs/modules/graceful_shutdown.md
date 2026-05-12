# Graceful Shutdown

> Handle SIGTERM/SIGINT with proper cleanup — coordinates shutdown across all components with a configurable grace period.

## Overview

The graceful shutdown module coordinates the shutdown of a Praxis instance. It provides a `ShutdownFlag` (an `Arc<AtomicBool>`) that can be checked by long-running components to know when to stop, and a `GracefulShutdown` coordinator that manages a list of `ShutdownHook` callbacks executed in reverse registration order.

Hooks can be implemented via the `ShutdownHook` trait or registered as closures via `register_closure`. The `wait_for_signal()` method is a platform-agnostic stub — production binaries should integrate with `tokio::signal::ctrl_c()` or the `ctrlc` crate.

**Current status:** Fully implemented. Used as a building block for daemon lifecycle management.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ShutdownFlag` | Cloneable `Arc<AtomicBool>` — shared shutdown signal across threads. |
| `ShutdownHook` | Trait with `name()` and `run() -> Result<(), String>`. Must be `Send + Sync`. |
| `GracefulShutdown` | Coordinator: holds flag, hooks list, and grace period (default 30s). |
| `NoOpHook` | Empty hook implementation for no-op registration. |

## Public API

### `ShutdownFlag`

```rust
impl ShutdownFlag {
    pub fn new() -> Self
    pub fn is_shutting_down(&self) -> bool
    pub fn request_shutdown(&self)
}
```

- Implements `Clone` (shares underlying `Arc`) and `Default`.

### `ShutdownHook` Trait

```rust
pub trait ShutdownHook: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self) -> Result<(), String>;
}
```

### `GracefulShutdown`

```rust
impl GracefulShutdown {
    pub fn new(grace_period: Duration) -> Self
    pub fn flag(&self) -> &ShutdownFlag
    pub fn register(&mut self, hook: BoxedHook)
    pub fn register_closure<F>(&mut self, name: &str, f: F)
        where F: Fn() -> Result<(), String> + Send + Sync + 'static
    pub fn run_hooks(&self) -> Vec<(String, Result<(), String>)>
    pub fn grace_period(&self) -> Duration
    pub fn wait_for_signal(&self)
}
```

- **`flag`** — Access the shared shutdown flag.
- **`register`** — Register a trait-based hook.
- **`register_closure`** — Register a closure as a hook (wraps in internal `ClosureHook` struct).
- **`run_hooks`** — Executes all hooks in reverse registration order (LIFO). Returns failed hook results. Non-fatal: all hooks run regardless of individual failures.
- **`wait_for_signal`** — Platform-agnostic stub (no-op). Override in binary with `tokio::signal::ctrl_c()` or similar.
- Default: grace period of 30 seconds.

### `NoOpHook`

```rust
pub struct NoOpHook;

impl ShutdownHook for NoOpHook {
    fn name(&self) -> &str { "no-op" }
    fn run(&self) -> Result<(), String> { Ok(()) }
}
```

## Configuration

No `praxis.toml` fields. Configured programmatically:

```rust
use praxis::graceful_shutdown::GracefulShutdown;
use std::time::Duration;
use std::sync::Arc;

let mut shutdown = GracefulShutdown::new(Duration::from_secs(60));

// Register cleanup hooks
shutdown.register_closure("save_state", || {
    // Persist application state
    Ok(())
});

shutdown.register_closure("close_connections", || {
    // Close database connections
    Ok(())
});

// In async context with Tokio:
// let flag = shutdown.flag().clone();
// tokio::spawn(async move {
//     tokio::signal::ctrl_c().await.unwrap();
//     flag.request_shutdown();
// });
// loop {
//     if shutdown.flag().is_shutting_down() { break; }
//     // ... work ...
// }
// shutdown.run_hooks();
```

## Dependencies

- **`std::sync::Arc`** — Shared flag ownership.
- **`std::sync::atomic::AtomicBool`** — Lock-free shutdown signaling.
- **`std::time::Duration`** — Grace period configuration.

## Status

- ✅ Cloneable `ShutdownFlag` with `Arc<AtomicBool>`
- ✅ `ShutdownHook` trait + closure-based `register_closure`
- ✅ Reverse-order hook execution (LIFO)
- ✅ Hook failure recording (non-aborting)
- ✅ Configurable grace period
- ✅ Platform-agnostic signal wait stub
- ✅ Comprehensive test coverage

## Source

`src/graceful_shutdown.rs`