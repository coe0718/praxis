# State

> Session state tracking across phase boundaries — the single source of truth for what happened and where to resume.

## Overview

The state module provides `SessionState` and `SessionPhase`, the data structures that track a running agent session from start to finish. Every phase transition updates the state and persists it to `session_state.json`, enabling crash recovery: if Praxis is interrupted during the Act phase, the next invocation detects an incomplete session and resumes from where it left off.

`SessionPhase` models the five stages of a session: Orient, Decide, Act, Reflect, and Sleep. A session always starts in Orient and transitions linearly to Sleep. The state records which goal and task were selected, tool invocation hashes (for loop guard), provider attempts, file read records, and the rendered context string.

## Architecture

### Key Types

| Type | Description |
|---|---|
| `SessionPhase` | Enum: `Orient`, `Decide`, `Act`, `Reflect`, `Sleep`. Implements `Display`. |
| `SessionState` | Serializable session tracker with phase, timestamps, goal/task selections, and diagnostic data |
| `FileReadRecord` | Tracks individual file reads with path, mtime, reason, and token estimate (for context dedup) |

### `SessionState` Fields

| Field | Type | Description |
|---|---|---|
| `current_phase` | `SessionPhase` | Current phase in the cycle |
| `started_at` | `DateTime<Utc>` | When the session was created |
| `updated_at` | `DateTime<Utc>` | Last modification time |
| `completed_at` | `Option<DateTime<Utc>>` | Set when session reaches Sleep |
| `selected_goal_id` | `Option<String>` | ID of the chosen goal (from GOALS.md) |
| `selected_goal_title` | `Option<String>` | Title of the chosen goal |
| `requested_task` | `Option<String>` | Operator-injected task override |
| `orientation_summary` | `Option<String>` | Summary produced by Orient phase |
| `action_summary` | `Option<String>` | Summary of the action taken |
| `last_outcome` | `Option<String>` | Outcome label (e.g., `goal_selected`, `review_failed`, `tool_executed`) |
| `resume_count` | `u32` | How many times this session was resumed after interruption |
| `selected_tool_name` | `Option<String>` | Name of the tool selected in Decide phase |
| `selected_tool_request_id` | `Option<i64>` | Approval request ID for tool execution |
| `tool_invocation_hashes` | `Vec<String>` | Hashes of tool invocations (for loop guard detection) |
| `provider_attempts` | `Vec<ProviderAttempt>` | LLM provider call attempts made during the session |
| `file_reads` | `Vec<FileReadRecord>` | Files read during context assembly |
| `repeated_reads_avoided` | `u32` | Count of deduplicated file reads |
| `context_sources` | `Vec<String>` | Names of context sources included in the assembled context |
| `rendered_context` | `Option<String>` | Full rendered context string (large, skipped in serialization if None) |

## Public API

### Constructors

```rust
SessionState::new(now: DateTime<Utc>, requested_task: Option<String>) -> Self
```

Create a fresh session starting at Orient phase.

### Persistence

```rust
SessionState::load(path: &Path) -> Result<Option<Self>>
SessionState::save(&self, path: &Path) -> Result<()>
```

Load from or save to `session_state.json`. Load returns `None` if the file doesn't exist.

### Phase Management

```rust
state.mark_phase(phase: SessionPhase, now: DateTime<Utc>)
state.finish(outcome: impl Into<String>, now: DateTime<Utc>)
state.is_incomplete() -> bool
```

`mark_phase()` advances to the next phase. `finish()` sets the phase to Sleep and records the outcome. `is_incomplete()` returns true if the session was interrupted before completing (i.e., not in Sleep and no `completed_at`).

### Helpers

```rust
state.selected_task_label() -> Option<String>
```

Returns the requested task if set, otherwise the selected tool name.

## Usage

The loop module is the primary consumer. It creates or loads state at session start, advances phases, and saves after every transition:

```rust
let mut state = self.load_or_create_state(now, task)?;
// ... orient phase ...
state.mark_phase(SessionPhase::Decide, now);
state.save(&paths.state_file)?;
```

For crash recovery:

```rust
if let Some(existing) = SessionState::load(&paths.state_file)? {
    if existing.is_incomplete() {
        // Resume from saved phase
    }
}
```

## Data Files

| File | Purpose |
|---|---|
| `session_state.json` | Serialized `SessionState` — the live session checkpoint |

## Dependencies

- **time** — `DateTime<Utc>` for timestamps
- **usage** — `ProviderAttempt` for provider call tracking

## Source

`src/state.rs`
