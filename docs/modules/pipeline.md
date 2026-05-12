# Pipeline

> Declarative tool composition and chaining — chain multiple tool calls where output from one step feeds as input to the next.

## Overview

The pipeline module provides a declarative system for composing multiple tool calls into reusable sequences. Each pipeline consists of ordered steps, where each step specifies a tool to execute, its arguments (supporting variable interpolation from previous step output, original input, or environment variables), and an optional condition. A failure policy controls behavior when a step fails — stop the pipeline, continue, run a rollback pipeline, or retry.

Variable references use `$` prefix syntax: `$prev.field` (previous step output), `$input.field` (original pipeline input), or `$env.VAR` (environment variable). Arguments are resolved at execution time via `PipelineContext`.

The registry (`PipelineRegistry`) stores named pipelines in a `HashMap` and provides CRUD operations.

**Current status:** Data model and variable resolution are fully implemented. Pipeline execution (executing each step, collecting results, applying failure policy) is a future extension — the module provides the registry, step model, context, and result types.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `Pipeline` | A named pipeline with ordered steps and a failure policy. |
| `PipelineStep` | A single step with tool name, args, optional condition, and timeout. |
| `FailurePolicy` | Enum: `Stop`, `Continue`, `Rollback(name)`, or `Retry { max_attempts, delay_secs }`. |
| `PipelineContext` | Execution context holding original input and step outputs; resolves variable references. |
| `PipelineRegistry` | Named pipeline registry with CRUD operations. |
| `StepResult` | Result of a single step execution (index, success, output, duration, error). |
| `PipelineResult` | Final result of a pipeline run (success, steps, total duration). |

### Relationships

`PipelineRegistry` owns a `HashMap<String, Pipeline>`. `Pipeline` contains `Vec<PipelineStep>`. `PipelineContext` is constructed per run and tracks `step_outputs` for `$prev` resolution.

## Public API

### `Pipeline`

```rust
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<PipelineStep>,
    pub on_failure: FailurePolicy,
    pub description: Option<String>,
}
```

### `PipelineStep`

```rust
pub struct PipelineStep {
    pub tool: String,
    pub args: HashMap<String, String>,
    pub condition: Option<String>,
    pub timeout_secs: u64,     // default: 300
}
```

### `FailurePolicy`

| Variant | Behavior |
|---------|----------|
| `Stop` (default) | Halt execution immediately. |
| `Continue` | Skip to next step. |
| `Rollback(String)` | Run the named rollback pipeline. |
| `Retry { max_attempts, delay_secs }` | Retry up to N times with delay between attempts. |

### `PipelineContext`

```rust
impl PipelineContext {
    pub fn new(input: HashMap<String, serde_json::Value>) -> Self
    pub fn resolve(&self, value: &str) -> String
    pub fn resolve_args(&self, args: &HashMap<String, String>) -> HashMap<String, serde_json::Value>
    pub fn record_output(&mut self, output: serde_json::Value)
}
```

- **`resolve`** — Resolves `$prev.field`, `$input.field`, `$env.VAR` references.
- **`resolve_args`** — Resolves all arg values and attempts JSON parsing.
- **`record_output`** — Records a step's output for subsequent `$prev` resolution.

### `PipelineRegistry`

```rust
impl PipelineRegistry {
    pub fn new() -> Self
    pub fn register(&mut self, pipeline: Pipeline)
    pub fn get(&self, name: &str) -> Option<&Pipeline>
    pub fn list(&self) -> Vec<String>
    pub fn remove(&mut self, name: &str) -> bool
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
}
```

### Result Types

```rust
pub struct StepResult {
    pub step_index: usize,
    pub tool: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub struct PipelineResult {
    pub name: String,
    pub success: bool,
    pub steps: Vec<StepResult>,
    pub total_duration_ms: u64,
}
```

## Configuration

Defined in `praxis.toml` as an array of pipeline definitions:

```toml
[[pipeline]]
name = "deploy"
description = "Build and deploy pipeline"
on_failure = "rollback"

[[pipeline.steps]]
tool = "git_pull"
args = { branch = "main" }
timeout_secs = 60

[[pipeline.steps]]
tool = "run_tests"
timeout_secs = 120

[[pipeline.steps]]
tool = "deploy"
args = { env = "$prev.branch" }
```

## Variable Reference Syntax

| Syntax | Resolves To |
|--------|-------------|
| `$prev.field` | Output from the previous step, field `field`. |
| `$input.field` | Original pipeline input, field `field`. |
| `$env.VAR` | Environment variable `VAR`. |
| *(no `$`)* | Literal string value. |

## Dependencies

- **`serde` / `serde_json`** — Serialization for pipeline structures and step outputs.
- **`HashMap`** — Pipeline registry and argument storage.

## Status

- ✅ Pipeline model + step model with serde
- ✅ Failure policy enum (Stop, Continue, Rollback, Retry)
- ✅ Variable resolution ($prev, $input, $env)
- ✅ Pipeline registry (CRUD)
- ✅ Serialization round-trip
- ⏳ Pipeline execution engine (step runner, hookup to tool system)

## Source

`src/pipeline.rs`
