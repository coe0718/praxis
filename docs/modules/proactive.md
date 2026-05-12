# Proactive

> Proactive agent wake-up scheduler and condition monitoring. The agent can schedule wake-ups, monitor conditions, and initiate actions without external prompts.

## Overview

The `proactive` module enables Praxis to act autonomously without waiting for operator input. It defines wake-up schedules with composite trigger conditions (time-based via cron, state-based, file-change, and webhook), and maps them to actions like running tools, sending messages, triggering skills, or starting routines.

A `ProactiveAgent` struct manages the wake-up list with priority ordering and a cooldown mechanism (60-second minimum between checks per wake-up). An optional `ProactiveConfig` controls enablement, check interval, and hourly rate limit.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `Condition` | Enum of trigger conditions: `Time` (cron), `State` (key/value), `FileChanged` (path exists), `Webhook` (endpoint), `And`, `Or`. |
| `WakeUp` | A scheduled proactive action: `id`, `name`, `condition`, `action`, `priority`, `enabled`. |
| `WakeAction` | Enum of actions: `RunTool`, `SendMessage`, `RunSkill`, `StartRoutine`. |
| `ProactiveAgent` | Scheduler managing `wake_ups` with `last_check` cooldown map. Sorted by descending priority. |
| `ProactiveConfig` | Configuration: `enabled` (default: false), `check_interval_seconds` (default: 60), `max_actions_per_hour` (default: 10). |

### Condition Checking

The `ProactiveAgent::check()` method evaluates each wake-up:
1. Skips disabled wake-ups.
2. Skips if checked within the last 60 seconds (cooldown).
3. Evaluates the `Condition` recursively (supports nested `And`/`Or`).
4. Returns IDs of triggered wake-ups and updates `last_check`.

Currently, `Time`, `State`, and `Webhook` conditions are placeholder-stubs that return `false`. Only `FileChanged` (checks if path exists) and composite `And`/`Or` conditions are fully functional.

## Public API

```rust
// Trigger conditions
pub enum Condition {
    Time { cron: String },
    State { key: String, expected: serde_json::Value },
    FileChanged { path: String },
    Webhook { endpoint: String },
    And(Vec<Condition>),
    Or(Vec<Condition>),
}

// Wake-up schedule
pub struct WakeUp {
    pub id: String,
    pub name: String,
    pub condition: Condition,
    pub action: WakeAction,
    pub priority: i32,
    pub enabled: bool,
}

// Actions
pub enum WakeAction {
    RunTool { name: String, args: serde_json::Value },
    SendMessage { channel: String, text: String },
    RunSkill { name: String, params: serde_json::Value },
    StartRoutine { name: String },
}

// Proactive agent
pub struct ProactiveAgent {
    pub wake_ups: Vec<WakeUp>,
    pub last_check: HashMap<String, i64>,
}
impl ProactiveAgent {
    pub fn new() -> Self;
    pub fn add_wake_up(&mut self, wake_up: WakeUp);
    pub fn check(&mut self) -> Vec<String>;
}

// Configuration
pub struct ProactiveConfig {
    pub enabled: bool,
    pub check_interval_seconds: u32,
    pub max_actions_per_hour: u32,
}
impl Default for ProactiveConfig { ... }

// Runner
pub async fn run_proactive_loop(mut agent: ProactiveAgent);
```

## Configuration

```toml
[proactive]
enabled = false
check_interval_seconds = 60
max_actions_per_hour = 10
```

### Example: File-Change Trigger

```rust
let mut agent = ProactiveAgent::new();
agent.add_wake_up(WakeUp {
    id: "watch-config".into(),
    name: "Config watcher".into(),
    condition: Condition::FileChanged { path: "/etc/praxis/praxis.toml".into() },
    action: WakeAction::RunTool {
        name: "reload-config".into(),
        args: serde_json::json!({}),
    },
    priority: 10,
    enabled: true,
});
```

## Dependencies

- `chrono` — timestamps and cooldown tracking
- `serde` / `serde_json` — serialization
- `tokio` — async interval loop

## Source

`src/proactive.rs`