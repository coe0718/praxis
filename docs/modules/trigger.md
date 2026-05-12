# Trigger

> Event triggers — webhook → tool execution without LLM. Direct tool execution triggered by events/webhooks, scheduled (cron) triggers, and conditional chains.

## Overview

The `trigger` module provides three trigger mechanisms that let Praxis execute tools without LLM involvement. **Event triggers** route webhook-style events to tool calls with wildcard pattern matching and composite conditions. **Scheduled triggers** use cron expressions to fire tools at specific times. **Trigger chains** link multiple tool executions where output from one step feeds into the next.

All triggers resolve to `ExecutingTool` structs that downstream systems can dispatch immediately.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `Event` | Inbound event payload: `event_type`, `source`, `payload` (JSON), `timestamp`. |
| `EventTrigger` | Maps an `event_pattern` (supports `*` and `prefix.*` wildcards) to a `tool_name` with `param_mapping`, `conditions`, and optional `transform`. |
| `TriggerCondition` | Enum for conditional logic: `Equals`, `Contains`, `Regex`, `GreaterThan`, `LessThan`, `And`, `Or`, `Not`. |
| `EventRouter` | Manages trigger rules and routes events to matching tools via `route()` (first match) or `route_all()` (all matches). |
| `WebhookHandler` | Wraps `EventRouter` with optional HMAC-SHA256 signature verification and a `handle()` convenience method. |
| `ExecutingTool` | Output of routing: `tool_name` + `args` (JSON map). |
| `ScheduledTrigger` | Cron-based trigger with `id`, `cron` (5-field), `tool_name`, `args`, `timezone`, `enabled`, and optional `emit_event`. |
| `ScheduledTriggerManager` | Manages scheduled triggers. `check(now)` evaluates cron expressions and returns due `ExecutingTool` instances. |
| `TriggerChain` | Ordered chain of steps where output of one feeds into the next, with `stop_on_failure` support. |
| `ChainStep` | A step in a chain: `tool_name` + `args` (supports `$prev.output.field` references). |

### Cron Expression Support

The `ScheduledTriggerManager` supports standard 5-field cron expressions:
- `*` — any value
- `?` — any value (alternative)
- `*/N` — every N units (step)
- `V1,V2` — comma-separated values
- `S-E` — numeric ranges
- Exact values for minute, hour, day, month, weekday

Weekday conversion: chrono Monday=0 is mapped to cron Monday=1 (Sunday = 7).

## Public API

```rust
// Event types
pub struct Event {
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

pub struct EventTrigger {
    pub event_pattern: String,
    pub tool_name: String,
    pub param_mapping: HashMap<String, String>,
    pub conditions: Vec<TriggerCondition>,
    pub transform: Option<serde_json::Value>,
}

pub enum TriggerCondition {
    Equals { field: String, value: serde_json::Value },
    Contains { field: String, value: String },
    Regex { field: String, pattern: String },
    GreaterThan { field: String, value: f64 },
    LessThan { field: String, value: f64 },
    And(Vec<TriggerCondition>),
    Or(Vec<TriggerCondition>),
    Not(Box<TriggerCondition>),
}

// Event router
pub struct EventRouter;
impl EventRouter {
    pub fn new() -> Self;
    pub fn add_trigger(&mut self, trigger: EventTrigger);
    pub fn remove_trigger(&mut self, event_pattern: &str) -> bool;
    pub fn route(&self, event: &Event) -> Option<ExecutingTool>;
    pub fn route_all(&self, event: &Event) -> Vec<ExecutingTool>;
}

// Executing tool (result)
pub struct ExecutingTool {
    pub tool_name: String,
    pub args: HashMap<String, serde_json::Value>,
}

// Webhook handler
pub struct WebhookHandler;
impl WebhookHandler {
    pub fn new(router: EventRouter) -> Self;
    pub fn with_secret(mut self, secret: &str) -> Self;
    pub fn handle(&self, event_type: &str, source: &str, payload: serde_json::Value) -> Option<ExecutingTool>;
    pub fn verify(&self, signature: &str, body: &[u8]) -> bool;
}

// Scheduled triggers
pub struct ScheduledTrigger {
    pub id: String,
    pub cron: String,
    pub tool_name: String,
    pub args: HashMap<String, serde_json::Value>,
    pub emit_event: Option<String>,
    pub timezone: String,
    pub enabled: bool,
    pub description: Option<String>,
}

pub struct ScheduledTriggerManager;
impl ScheduledTriggerManager {
    pub fn new() -> Self;
    pub fn add(&mut self, trigger: ScheduledTrigger);
    pub fn remove(&mut self, id: &str) -> bool;
    pub fn list(&self) -> &[ScheduledTrigger];
    pub fn check(&self, now: DateTime<Utc>) -> Vec<ExecutingTool>;
}

// Trigger chains
pub struct TriggerChain {
    pub id: String,
    pub steps: Vec<ChainStep>,
    pub stop_on_failure: bool,
}

pub struct ChainStep {
    pub tool_name: String,
    pub args: HashMap<String, String>,
}
```

## Configuration

Event triggers and scheduled triggers are defined programmatically via `EventRouter` / `ScheduledTriggerManager`. No `praxis.toml` section exists for this module — triggers are added at runtime.

### Example: Event Trigger

```rust
let mut router = EventRouter::new();
router.add_trigger(EventTrigger {
    event_pattern: "github.push".into(),
    tool_name: "deploy".into(),
    param_mapping: HashMap::from([
        ("branch".into(), "ref".into()),
        ("repo".into(), "repository.full_name".into()),
    ]),
    conditions: vec![],
    transform: None,
});
```

### Example: Scheduled Trigger

```rust
let mut mgr = ScheduledTriggerManager::new();
mgr.add(ScheduledTrigger {
    id: "daily-healthcheck".into(),
    cron: "0 9 * * *".into(),
    tool_name: "health_check".into(),
    args: HashMap::new(),
    emit_event: None,
    timezone: "UTC".into(),
    enabled: true,
    description: Some("Daily health check at 9 AM UTC".into()),
});
```

### Example: Trigger Chain

```rust
let chain = TriggerChain {
    id: "deploy_pipeline".into(),
    steps: vec![
        ChainStep { tool_name: "build".into(), args: HashMap::new() },
        ChainStep { tool_name: "deploy".into(), args: HashMap::from([
            ("artifact".into(), "$prev.output.path".into()),
        ])},
    ],
    stop_on_failure: true,
};
```

## Dependencies

- `chrono` — timestamp and cron field evaluation
- `hmac` + `sha2` — webhook signature verification
- `hex` — HMAC hex encoding/decoding
- `regex` — condition regex matching
- `serde` / `serde_json` — serialization
- `log` — diagnostic logging

## Source

`src/trigger.rs`