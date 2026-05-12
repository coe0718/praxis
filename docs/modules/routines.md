# Routines

> Event-driven self-healing background workers. Beyond basic cron with event-driven triggers, webhook-reactive jobs, and self-healing background workers with heartbeat monitors.

## Overview

The `routines` module provides a lightweight background worker engine in Praxis. A `Routine` is defined by an ID, a human-readable name, optional cron schedule, event trigger list, an action, a heartbeat timeout, and an enabled flag. The `RoutinesEngine` manages registration and event-based triggering.

The module includes infrastructure for heartbeat monitoring — each routine tracks execution start time and logs completion duration. A `start_heartbeat_monitor` async loop checks for stuck routines at 60-second intervals.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `Routine` | A background routine: `id`, `name`, `cron` (optional), `triggers` (event types), `action`, `heartbeat_timeout`, `enabled`. |
| `RoutineAction` | Action specification: `type_` (e.g., "tool", "script") and `params` (JSON map). |
| `RoutinesEngine` | Scheduler managing routines in a `HashMap<String, Routine>` with event-based dispatch. |

### Execution Flow

1. `RoutinesEngine::trigger(event, payload)` iterates all routines.
2. For each enabled routine matching the event type, `execute_routine()` is called.
3. Execution starts a timer, runs the action (currently a placeholder), and logs elapsed time.
4. `start_heartbeat_monitor()` runs a loop that checks for routines exceeding their heartbeat timeout.

Note: The action execution logic is currently a placeholder — real tool dispatch requires integration with the tool system.

## Public API

```rust
// Routine definition
pub struct Routine {
    pub id: String,
    pub name: String,
    pub cron: Option<String>,
    pub triggers: Vec<String>,
    pub action: RoutineAction,
    pub heartbeat_timeout: u64,
    pub enabled: bool,
}

pub struct RoutineAction {
    pub type_: String,
    pub params: HashMap<String, serde_json::Value>,
}

// Routines engine
pub struct RoutinesEngine;
impl RoutinesEngine {
    pub fn new() -> Self;
    pub fn register(&mut self, routine: Routine);
    pub async fn trigger(&self, event: &str, payload: serde_json::Value) -> Result<()>;
    pub async fn start_heartbeat_monitor(&self);
}
```

## Configuration

No `praxis.toml` section. Routines are registered programmatically via `RoutinesEngine::register()`.

### Example

```rust
let mut engine = RoutinesEngine::new();
engine.register(Routine {
    id: "log-cleanup".into(),
    name: "Log Cleanup".into(),
    cron: Some("0 3 * * *".into()), // 3 AM daily
    triggers: vec!["disk.warning".into()],
    action: RoutineAction {
        type_: "tool".into(),
        params: HashMap::from([
            ("tool".into(), serde_json::json!("cleanup-logs")),
        ]),
    },
    heartbeat_timeout: 300,
    enabled: true,
});

// Trigger by event
engine.trigger("disk.warning", serde_json::json!({"usage": 95})).await?;

// Start monitor
tokio::spawn(engine.start_heartbeat_monitor());
```

## Dependencies

- `tokio` — async runtime and interval timers
- `anyhow` — error handling
- `serde` / `serde_json` — serialization
- `log` — execution metrics logging

## Source

`src/routines.rs`