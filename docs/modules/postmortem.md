# Postmortem

> Automatic markdown postmortem generation for reviewer failures, eval failures, and blocked-loop-guard events.

## Overview

The postmortem module automatically generates structured Markdown documentation whenever a Praxis session ends with a degraded outcome. Its purpose is to leave a persistent, human-readable audit trail that the operator can review to understand *why* things went wrong, *what* the agent was trying to do, and *what* the reviewer and evaluation systems found.

Postmortems are only recorded for sessions that meet specific failure criteria — not every session gets one. This keeps the signal-to-noise ratio high: when a postmortem exists, it means something worth investigating happened.

## Architecture

### Trigger conditions

A postmortem is recorded when any of the following is true:

| Condition | Description |
|-----------|-------------|
| `review_failed` | The agent loop's review gate rejected the session's output. |
| `eval_failed` | One or more operator-defined evaluations failed. |
| `blocked_loop_guard` | The loop guard detected and stopped repeated/stuck behavior. |
| `eval_failures > 0` | Any eval run in the session had a `"failed"` status. |

### Postmortem format

Each postmortem is appended as a level-2 section to `POSTMORTEMS.md`:

```markdown
## Session abc123 - 2026-05-04T18:30:00Z
- Outcome: review_failed
- Goal: 42: Fix authentication bug
- Task: Refactor the auth middleware
- Summary: Applied changes to src/auth.rs
- Review: The changes introduced a regression in token validation
- Eval summary: passed=3 failed=1 skipped=0 trust_failures=0
- Trigger: review gate failed
### Findings
- Missing error handling for expired tokens
- Test coverage gap in refresh flow
### Failed Evals
- auth-refresh-test (eval-004)
```

### Key functions

| Function | Description |
|----------|-------------|
| `append_postmortem()` | Main entry point. Takes session data, outcome, review summary, findings, and eval results. Appends a formatted section to the postmortems file. |
| `should_record()` | Internal predicate — returns `true` if the outcome warrants a postmortem. |
| `trigger_label()` | Maps outcome strings to human-readable trigger descriptions. |

## Public API

```rust
use crate::postmortem::append_postmortem;

append_postmortem(
    &paths,           // PraxisPaths — locates POSTMORTEMS.md
    &session,         // StoredSession — session metadata (ID, goal, task, etc.)
    "review_failed",  // outcome string
    "Changes introduced a regression", // review summary
    &findings,        // Vec<String> — reviewer findings
    eval_summary,     // EvalSummary — passed/failed/skipped/trust_failures counts
    &eval_results,    // Vec<EvalOutcome> — individual eval results
)?;
```

The function silently returns `Ok(())` if the session outcome doesn't meet the recording threshold.

## Configuration

No configuration required. Postmortem recording is always active and cannot be disabled.

### Feature flags

No feature flag — always compiled.

## Usage

This module is called automatically by the reflect phase in `PraxisRuntime::execute_reflect()`. There is no CLI command to manually trigger a postmortem.

To review past postmortems:

```bash
cat /path/to/data_dir/POSTMORTEMS.md
```

Or use `praxis status` which surfaces a summary of recent anomalies and failures.

## Data Files

| File | Location | Description |
|------|----------|-------------|
| `POSTMORTEMS.md` | `{data_dir}/POSTMORTEMS.md` | Append-only Markdown file. Created automatically on first postmortem. Each session's postmortem is a new `##` section. |

## Dependencies

### Internal Praxis modules

- **`paths`** — `PraxisPaths` for locating the output file.
- **`quality`** — `EvalSummary`, `EvalOutcome` for evaluation results.
- **`storage`** — `StoredSession` for session metadata.

### External crates

- **`anyhow`** — error handling for file I/O.

## Source

`src/postmortem.rs`
