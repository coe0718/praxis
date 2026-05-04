# Argus

> Session analytics engine — drift detection, failure clustering, repeated-work patterns, token hotspots, and actionable directives.

## Overview

The `argus` module is Praxis's observability and analytics layer. Named after the many-eyed guardian of Greek mythology, Argus analyzes recent session history to detect quality drift, cluster failure modes, identify recurring work that should be automated, and pinpoint the most expensive token usage paths. Its output drives both the learning system (opportunity mining) and the operator's morning brief.

Argus queries the SQLite database directly (not through the store trait) to build a comprehensive report from the last N sessions. The report includes actionable directives — plain-English recommendations that are injected into the Orient context to influence agent behavior.

## Architecture

### Main Entry Point (`mod.rs`)

```rust
pub fn analyze(database_file: &Path, limit: usize) -> Result<ArgusReport>
```

Loads recent sessions, runs all analysis passes, and assembles the `ArgusReport`.

### Types

| Type | Description |
|------|-------------|
| `ArgusReport` | Full analysis output: session count, failure counts, drift report, repeated work patterns, failure clusters, token hotspots, and directives. |
| `DriftStatus` | `InsufficientData`, `Stable`, `Regressed`, `Improving`. |

### Submodules

#### `drift.rs` — Quality Drift Detection

Compares the recent window of sessions against a baseline window. The "drift score" is a weighted combination of review failure rate (50%), eval failure rate (30%), and loop-guard block rate (20%). A delta ≥ 0.20 indicates regression; ≤ -0.20 indicates improvement.

| Type | Description |
|------|-------------|
| `DriftReport` | `status`, `recent_score`, `baseline_score`. |
| `DriftStatus` | Four-state classification of quality trajectory. |

Requires at least `2 × window` sessions to produce a result; otherwise returns `InsufficientData`.

#### `patterns.rs` — Pattern Detection

| Type | Description |
|------|-------------|
| `SessionRow` | Lightweight session data: day, outcome, failure counts, goal/task info. |
| `RepeatedWorkPattern` | A recurring task/goal pattern: label, session count, distinct days, latest outcome. |

| Function | Description |
|----------|-------------|
| `recent_sessions()` | Queries the database for the last N sessions as `SessionRow` structs. |
| `cluster_failures()` | Groups non-success sessions by outcome string, sorted by frequency. |
| `repeated_work_patterns()` | Finds tasks/goals that appear ≥ 2 times across sessions, sorted by distinct days. |
| `token_hotspots()` | Queries the `token_ledger` table for the top 3 (phase, provider) pairs by total token usage. |

#### `render.rs` — Report Rendering

| Function | Description |
|----------|-------------|
| `render()` | Formats the full `ArgusReport` as a human-readable key-value string. |
| `directives()` | Generates plain-English action recommendations based on report findings. |

**Directive triggers:**

| Condition | Directive |
|-----------|-----------|
| Drift regressed | "Pause ambitious changes and repair the recurring failure path." |
| Review failures > 0 | "Tighten completion discipline before Reflect." |
| Eval failures > 0 | "Inspect recent eval regressions before changing habits." |
| Loop guard blocks > 0 | "Diversify tool plans instead of repeating identical invocations." |
| Waiting sessions > 0 | "Promote prerequisites or satisfy wake conditions." |
| Repeated work (≥2 days) | "Promote recurring work into automation or a parent goal." |
| Token hotspot found | "Trim the hottest phase first when chasing token savings." |
| Repeated reads avoided | "Expand anatomy coverage around frequently revisited files." |
| All clear | "Recent sessions look stable — keep the current pattern." |

## Public API

```rust
pub fn analyze(database_file: &Path, limit: usize) -> Result<ArgusReport>;

pub use drift::DriftStatus;
pub use render::render;
```

## Configuration

No `praxis.toml` section is needed. The session limit (number of recent sessions to analyze) is passed as a parameter at the call site (typically 10). The drift window size is hardcoded at 5.

## Usage

```bash
# Run Argus analysis and view the report
praxis argus analyze

# The report is also used internally by the learning system and morning brief
```

## Data Files

| Source | Description |
|--------|-------------|
| SQLite `sessions` table | Session outcomes, failure counts, goal/task selections. |
| SQLite `token_ledger` table | Per-phase, per-provider token usage records. |

Argus is read-only — it never writes to the database.

## Dependencies

- `rusqlite` — direct database access
- `serde_json` — evidence serialization in opportunities
- `paths` — database file path
- `storage` — `SqliteSessionStore` (used by the `learning` module's Argus integration)

## Source

`src/argus/` — `mod.rs`, `drift.rs`, `patterns.rs`, `render.rs`
