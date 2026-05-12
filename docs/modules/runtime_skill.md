# Runtime Skill

> Runtime skill creation — Agent creates its own skills at runtime. Allows Praxis to dynamically generate and register new skills during execution.

## Overview

The `runtime_skill` module enables Praxis to create skills on the fly — no restarts, no file system edits. A `RuntimeSkillFactory` manages a collection of `RuntimeSkill` instances, each with a `SkillTrigger` (pattern match, tool invocation, cron schedule, or event type) and a list of `SkillAction` steps (run a tool, set a context variable, send a message, or invoke another skill).

Skills are created from a `SkillSpec` specification, get a unique ID (`rt_<timestamp_nanos>`), and are stored in-memory. The factory provides `create()`, `get()`, `list()`, and `remove()` operations.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `RuntimeSkill` | A runtime-generated skill: `id`, `name`, `description`, `trigger`, `actions`, `created_at`. |
| `SkillTrigger` | Enum: `Pattern` (message match), `Tool` (tool invocation), `Cron` (scheduled), `Event` (event type). |
| `SkillAction` | Enum: `RunTool`, `Set` (context variable), `Message` (send to channel), `InvokeSkill`. |
| `RuntimeSkillFactory` | Factory managing runtime skills in a `HashMap<String, RuntimeSkill>`. |
| `SkillSpec` | Specification for creating a new skill: `name`, `description`, `trigger`, `actions`. |

## Public API

```rust
// Runtime skill
pub struct RuntimeSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger: SkillTrigger,
    pub actions: Vec<SkillAction>,
    pub created_at: i64,
}

// Trigger types
pub enum SkillTrigger {
    Pattern(String),
    Tool(String),
    Cron(String),
    Event(String),
}

// Action types
pub enum SkillAction {
    RunTool { name: String, args: serde_json::Value },
    Set { key: String, value: serde_json::Value },
    Message { channel: String, text: String },
    InvokeSkill { id: String, params: serde_json::Value },
}

// Factory
pub struct RuntimeSkillFactory;
impl RuntimeSkillFactory {
    pub fn new() -> Self;
    pub fn create(&mut self, spec: SkillSpec) -> Result<String, anyhow::Error>;
    pub fn get(&self, id: &str) -> Option<&RuntimeSkill>;
    pub fn list(&self) -> Vec<&RuntimeSkill>;
    pub fn remove(&mut self, id: &str) -> bool;
}

// Skill specification
pub struct SkillSpec {
    pub name: String,
    pub description: String,
    pub trigger: SkillTrigger,
    pub actions: Vec<SkillAction>,
}
```

## Configuration

No `praxis.toml` section. Runtime skills are created and managed in memory via `RuntimeSkillFactory`.

### Example

```rust
let mut factory = RuntimeSkillFactory::new();

let spec = SkillSpec {
    name: "greeting".into(),
    description: "Respond to greetings".into(),
    trigger: SkillTrigger::Pattern("hello".into()),
    actions: vec![SkillAction::Message {
        channel: "general".into(),
        text: "Hello! How can I help?".into(),
    }],
};

let skill_id = factory.create(spec).unwrap();
let skill = factory.get(&skill_id).unwrap();
assert_eq!(skill.name, "greeting");

// List all runtime skills
for skill in factory.list() {
    println!("{}: {}", skill.id, skill.name);
}
```

## Dependencies

- `chrono` — creation timestamps
- `serde` / `serde_json` — serialization
- `anyhow` — error handling

## Source

`src/runtime_skill.rs`