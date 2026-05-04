# Examples

> Synthetic training example generation from completed sessions.

## Overview

The `examples` module produces lightweight `(context, action, outcome)` training triples from completed agent sessions and appends them to `evals/examples.jsonl`. These examples serve as reusable signals for few-shot prompting during Orient/Act phases, offline fine-tuning datasets, and regression tests against known-good trajectories.

Only sessions with "useful" outcomes are recorded — idle or skipped sessions are filtered out. The file is capped at 500 records and pruned automatically when the limit is exceeded.

## Architecture

### Types

| Type | Description |
|------|-------------|
| `SyntheticExample` | A single training triple: `id`, `generated_at`, `context`, `action`, `outcome`, `session_id`, `goal_id`, `quality_score`. |

### Functions

| Function | Description |
|----------|-------------|
| `SyntheticExample::new()` | Creates an example with an auto-generated ID (`<timestamp>-<outcome-slug>`). |
| `with_session_id()` | Builder method to attach the source session ID. |
| `with_goal_id()` | Builder method to attach the source goal ID. |
| `with_quality_score()` | Builder method to attach a clamped [0.0, 1.0] quality score. |
| `record_example()` | Appends an example to the JSONL file and prunes if over cap. |
| `load_recent_examples()` | Loads the last N examples (most recent first). |
| `examples_file()` | Returns the path `evals/examples.jsonl` from `PraxisPaths`. |
| `build_context()` | Assembles a context string from goal title, action summary, memory hits, and tool call count. |
| `is_useful_outcome()` | Returns `false` for empty, "idle", or "skipped" outcomes. |

## Public API

```rust
pub struct SyntheticExample {
    pub id: String,
    pub generated_at: DateTime<Utc>,
    pub context: String,
    pub action: String,
    pub outcome: String,
    pub session_id: Option<i64>,
    pub goal_id: Option<String>,
    pub quality_score: Option<f64>,
}

pub fn record_example(path: &Path, example: &SyntheticExample) -> Result<()>;
pub fn load_recent_examples(path: &Path, limit: usize) -> Result<Vec<SyntheticExample>>;
pub fn examples_file(paths: &PraxisPaths) -> PathBuf;
pub fn build_context(goal_title: Option<&str>, action_summary: &str, memory_hits: usize, tool_calls: usize) -> String;
pub fn is_useful_outcome(outcome: &str) -> bool;
```

## Configuration

No configuration is required. The retention cap is hardcoded at 500 records (`MAX_EXAMPLES`).

## Usage

Examples are generated automatically during the Reflect phase of the agent loop. No CLI command exists for manual example creation — they are an internal byproduct of session reflection.

```json
// Example JSONL record
{
  "id": "20260101T120000Z-review-success",
  "generated_at": "2026-01-01T12:00:00Z",
  "context": "Goal: fix CI flakiness. Tool calls: read_file, write_file. Memory hits: 3.",
  "action": "Identified race condition in test setup; patched parallel teardown.",
  "outcome": "success",
  "goal_id": "goal-123",
  "session_id": 42,
  "quality_score": 0.9
}
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `evals/examples.jsonl` | JSONL | Up to 500 synthetic training triples, auto-pruned. |

## Dependencies

- `paths` — `PraxisPaths` for file location

## Source

`src/examples.rs`
