# Quality

> Multi-layer quality assurance: shell-based reviewers, eval suites, generator-evaluator loops, and deterministic quality gates.

## Overview

The `quality` module provides Praxis's quality assurance stack, operating at four distinct levels:

1. **Reviewer** (`reviewer.rs`) — Runs shell-based verification commands against goal-specific criteria files. For each goal, a `goals_criteria/<goal_id>.json` file defines `done_when` conditions and shell commands. The reviewer executes them within a bounded resource budget and reports pass/fail with findings.

2. **Eval Suite** (`evals.rs`) — An operator-defined regression test suite stored as JSON files in `evals/`. Each eval defines a scenario, expected and forbidden behaviors, and shell commands to verify compliance. Evals can be triggered `always` (every session) or only during `canary` runs.

3. **Evaluate Loop** (`evaluate.rs`) — A formal generator→evaluator orchestration for in-flight content refinement. The generator produces output, the evaluator checks it against criteria, and failures feed structured feedback back to the generator for the next round. Terminates on pass or after `max_rounds` (default 3).

4. **Quality Gates** (`gates.rs`) — Lightweight deterministic checks applied before outputs become operator-visible. Gates are fast, non-LLM checks that can pass, redact, block, or request retry with feedback. The default delivery pipeline includes a non-empty check and credential scrubbing.

## Architecture

### Submodules

#### `reviewer.rs` — Goal Completion Reviewer

| Type | Description |
|------|-------------|
| `ReviewerBudget` | Resource ceiling: `max_commands` (default 20), `max_output_bytes` (default 2048). |
| `GoalCriteria` | Per-goal verification: `goal_id`, `done_when` conditions, `verify_with` (always "shell"), `commands`. |
| `ReviewOutcome` | Result: `status` (Passed/Failed/Skipped), `summary`, `findings`. |
| `LocalReviewer` | Shell-based reviewer implementation. |
| `Reviewer` (trait) | `validate()` to check criteria files, `review()` to execute them. |

Commands are sandboxed to an allowlist: `git`, `grep`, `test`, `diff`, `wc`, `cat`, `echo`, `ls`, `find`, `cargo`, `true`, `false`, `exit`.

#### `evals.rs` — Eval Suite Runner

| Type | Description |
|------|-------------|
| `EvalDefinition` | An eval scenario: `id`, `name`, `when` trigger, `severity`, `scenario`, `expected_behavior`, `forbidden_behavior`, `relevant_memories`, `verify_with`, `commands`. |
| `EvalOutcome` | Result of running one eval: `eval_id`, `name`, `severity`, `status`, `summary`. |
| `EvalSummary` | Aggregate: passed/failed/skipped/trust_failures counts. |
| `EvalTrigger` | `Always` or `Canary`. |
| `LocalEvalSuite` | Default runner that loads and executes JSON eval definitions. |
| `EvalRunner` (trait) | `validate()` and `run()`. |

Eval commands use the same allowlist as the reviewer (minus `cargo`).

#### `evaluate.rs` — Generator/Evaluator Loop

| Type | Description |
|------|-------------|
| `EvaluateConfig` | `max_rounds` (default 3), `task_name`. |
| `GeneratorOutput` | Generated content plus optional metadata. |
| `EvaluatorVerdict` | `Pass` or `Fail(Vec<String>)` with feedback. |
| `EvaluateResult` | Final content, pass/fail, rounds attempted, remaining feedback. |

`run_evaluate_loop()` takes two closures (generator and evaluator) and iterates until pass or exhaustion.

#### `gates.rs` — Quality Gate Pipeline

| Type | Description |
|------|-------------|
| `GateDecision` | `Pass`, `Redact(String)`, `Block(String)`, `RetryWithFeedback(String)`. |
| `QualityGate` (trait) | `name()` and `check(content) -> GateDecision`. |
| `GatePipeline` | Ordered sequence of gates. First non-Pass wins. |
| `CredentialScrubGate` | Redacts `sk-ant-...`, `sk-...`, `Bearer ...` patterns. |
| `MaxLengthGate` | Returns `RetryWithFeedback` if content exceeds `max_bytes`. |
| `ForbiddenPhraseGate` | Blocks content containing any listed substring. |
| `NonEmptyGate` | Blocks whitespace-only content. |

`default_delivery_pipeline()` returns `NonEmptyGate → CredentialScrubGate`.

## Public API

```rust
// Reviewer
pub use reviewer::{GoalCriteria, LocalReviewer, ReviewOutcome, Reviewer, ReviewerBudget};

// Evals
pub use evals::{EvalDefinition, EvalOutcome, EvalRunner, EvalSummary, EvalTrigger, LocalEvalSuite, summarize};

// Evaluate loop
pub use evaluate::{EvaluateConfig, EvaluateResult, EvaluatorVerdict, GeneratorOutput, run_evaluate_loop};

// Gates
pub use gates::{
    CredentialScrubGate, ForbiddenPhraseGate, GateDecision, GatePipeline,
    MaxLengthGate, NonEmptyGate, QualityGate, default_delivery_pipeline,
};
```

## Configuration

| File | Purpose |
|------|---------|
| `goals_criteria/<goal_id>.json` | Per-goal shell-based verification criteria. |
| `evals/*.json` | Operator-defined eval definitions (trigger, severity, commands). |

Eval `severity` maps to the `EvalSeverity` enum used by the storage layer (e.g. `TrustDamaging`). The reviewer budget is hardcoded but can be overridden programmatically.

## Usage

```bash
# Quality runs are triggered automatically during Reflect
praxis run --once

# View recent session quality
praxis insights

# Canary mode runs all evals including canary-triggered ones
praxis canary run
```

## Data Files

| File/Directory | Format | Purpose |
|----------------|--------|---------|
| `goals_criteria/*.json` | JSON | Per-goal shell verification criteria. |
| `evals/*.json` | JSON | Eval scenario definitions. |
| `evals/examples.jsonl` | JSONL | Synthetic training examples (from the `examples` module). |

## Dependencies

- `storage` — `ReviewStatus`, `EvalSeverity`, `EvalStatus` types
- `paths` — `PraxisPaths` for criteria and eval directories

## Source

`src/quality/` — `mod.rs`, `reviewer.rs`, `evals.rs`, `evaluate.rs`, `gates.rs`
