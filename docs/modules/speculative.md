# Speculative

> Compare multiple rehearsed plan branches before committing to the safest or highest-yield action.

## Overview

The speculative execution module provides a lightweight scoring framework for when the Act phase needs to choose between multiple plausible approaches to a goal. Rather than committing to the first plan the LLM generates, the runtime can rehearse two or more branches, score each against success criteria and trust constraints, and then commit to the branch most likely to succeed.

This is especially valuable when the cheapest safe strategy is "try two narrow approaches in rehearsal, not one large irreversible one." A branch that mentions "force push to production" can be penalised even if it otherwise matches success criteria, while a branch that covers all required ground without violations wins.

The scoring model is intentionally simple — keyword and phrase matching against plan text — and is designed as a first-class primitive that a future evaluator backend can replace with semantic scoring.

**Current status:** Fully implemented and wired into the Act phase. `run_speculative()` in `phases.rs` generates a conservative alternative branch, scores both branches against success criteria and trust constraints, and commits to the higher-scoring approach. Gated by `LiteCapability::Speculative`.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `SpeculativeBranch` | A candidate action plan with an `id`, `description`, and `plan_text`. |
| `BranchScore` | Score assigned to a branch: criteria matched, trust violations, and a composite score. |
| `SpeculativeResult` | The outcome of a selection pass: the winning branch, all scores, and a human-readable rationale. |

### Scoring Model

- **Success criteria** — phrases that *should* appear in the plan text. Each match adds +1 to the score.
- **Trust constraints** — phrases whose presence *penalises* a branch (e.g. `"delete production"`, `"force push"`). Each match subtracts 1.
- **Composite score** — `criteria_matched − trust_violations`. May be negative.
- **Tie-breaking** — earliest branch in the input list wins.

## Public API

### `SpeculativeBranch`

```rust
pub fn new(id: impl Into<String>, description: impl Into<String>, plan_text: impl Into<String>) -> Self
```

Constructs a new branch candidate.

### `select_branch`

```rust
pub fn select_branch(
    branches: Vec<SpeculativeBranch>,
    success_criteria: &[String],
    trust_constraints: &[String],
) -> Option<SpeculativeResult>
```

Evaluates all branches against the given criteria and constraints, returning the winning branch and full scoring breakdown. Returns `None` if `branches` is empty.

### `SpeculativeResult`

| Field | Type | Description |
|-------|------|-------------|
| `winner` | `SpeculativeBranch` | The selected branch. |
| `scores` | `Vec<BranchScore>` | Scores for all branches in input order. |
| `rationale` | `String` | Human-readable explanation of the selection. |

### `BranchScore`

| Field | Type | Description |
|-------|------|-------------|
| `branch_id` | `String` | Identifier of the branch. |
| `criteria_matched` | `usize` | Number of success criteria matched. |
| `criteria_total` | `usize` | Total success criteria evaluated. |
| `trust_violations` | `usize` | Number of trust constraints that fired. |
| `score` | `i32` | Composite score (may be negative). |

## Configuration

No configuration files, environment variables, or feature flags. The module is always compiled in and available.

## Usage

```rust
use praxis::speculative::{SpeculativeBranch, select_branch};

let branches = vec![
    SpeculativeBranch::new("safe", "Careful approach", "Run tests and check logs carefully"),
    SpeculativeBranch::new("risky", "Fast approach", "Run tests and force push to production"),
];

let criteria = vec!["tests".to_string(), "logs".to_string()];
let constraints = vec!["force push".to_string()];

if let Some(result) = select_branch(branches, &criteria, &constraints) {
    println!("Winner: {}", result.winner.id);
    println!("Rationale: {}", result.rationale);
}
```

## Data Files

None. The speculative module operates purely in memory — no files are read or written.

## Dependencies

None. This is a standalone utility module with no dependencies on other Praxis modules.

## Source

`src/speculative/mod.rs`
