# Learning

> Daily learning runtime that mines session history for improvement opportunities and captures knowledge from source files.

## Overview

The `learning` module is Praxis's self-improvement engine. It runs once per day (throttled) and performs two main tasks: **source processing** (detecting changes in learning source files and capturing summaries) and **opportunity mining** (analyzing the Argus report for patterns that suggest improvements, such as quality drift or recurring work that should be automated).

Opportunities are deduplicated by signature and rate-limited (2 per day, 5 per week). Accepted opportunities are automatically promoted to goals in `GOALS.md`. A human-readable `PROPOSALS.md` file is kept in sync as a dashboard for the operator.

## Architecture

### Core Types (`mod.rs`)

| Type | Description |
|------|-------------|
| `StoredLearningSource` | A tracked source file with path, modification time, byte size, summary, and last-processed timestamp. |
| `StoredLearningRun` | Record of a completed learning cycle: sources processed, changed, opportunities created, notes. |
| `OpportunityStatus` | `Pending`, `Accepted`, or `Dismissed`. |
| `StoredOpportunity` | A persisted opportunity with signature (for dedup), kind, title, summary, status, and optional linked goal. |
| `OpportunityCandidate` | A new opportunity before it is persisted, with evidence JSON. |
| `LearningRunSummary` | The result of a `run_once()` call: counts, throttle status, and notes. |
| `OpportunityActionResult` | The result of accepting/dismissing an opportunity, including whether a goal was created. |
| `LearningNoteResult` | The result of appending a manual learning note. |

### Submodules

| Module | Description |
|--------|-------------|
| `sources.rs` | Scans `learning_sources/` for changed `.md`/`.txt` files, summarizes content, appends to the learnings log, and upserts tracking records. |
| `opportunities.rs` | Mines the `ArgusReport` for improvement candidates: drift regression triggers a stability opportunity; repeated work patterns (≥2 distinct days) trigger automation opportunities. Rate-limited by daily/weekly caps. |
| `entries.rs` | Appends structured markdown entries to `learnings.md` for both source-derived and manual/operator learning notes. |
| `proposals.rs` | Regenerates `PROPOSALS.md` from all opportunities grouped by status (Pending, Accepted, Dismissed). |
| `render.rs` | Renders learning run summaries, opportunities, notes, and lists as human-readable CLI output. |

## Public API

```rust
// Main entry point — run the daily learning cycle
pub fn run_once(paths: &PraxisPaths, store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<LearningRunSummary>;

// Update an opportunity's status (accept/dismiss)
pub fn update_opportunity(
    paths: &PraxisPaths, store: &SqliteSessionStore, id: i64,
    status: OpportunityStatus, now: DateTime<Utc>,
) -> Result<Option<OpportunityActionResult>>;

// Append a manual learning note
pub fn append_note(paths: &PraxisPaths, summary: &str, now: DateTime<Utc>) -> Result<LearningNoteResult>;

// CLI rendering helpers
pub use render::{render_action, render_list, render_note, render_run};
```

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| Daily opportunity limit | 2 | Max new opportunities created per calendar day. |
| Weekly opportunity limit | 5 | Max new opportunities created per 7-day window. |
| Source file extensions | `.md`, `.txt` | File types scanned in `learning_sources/`. |

These limits are hardcoded in `opportunities.rs` and not currently configurable via `praxis.toml`.

## Usage

```bash
# Run the learning cycle manually
praxis learn run

# List all opportunities
praxis learn list

# Accept an opportunity (promotes to goal)
praxis learn accept --id 3

# Dismiss an opportunity
praxis learn dismiss --id 4

# Append a manual learning note
praxis learn note "Switched to structured logging for better filtering"
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `learnings.md` | Markdown | Append-only log of source-derived and manual learning entries. |
| `PROPOSALS.md` | Markdown | Rendered view of all opportunities by status. Regenerated each cycle. |
| `learning_sources/` | Directory | `.md`/`.txt` files treated as learning source material. |
| SQLite tables | DB | `learning_sources`, `learning_runs`, `opportunities`. |

## Dependencies

- `argus` — provides the `ArgusReport` for opportunity mining
- `identity` — `ensure_goal()` promotes accepted opportunities to goals in `GOALS.md`
- `paths` — `PraxisPaths` for all file locations
- `storage` — `SqliteSessionStore` for persistence

## Source

`src/learning/` — `mod.rs`, `sources.rs`, `opportunities.rs`, `entries.rs`, `proposals.rs`, `render.rs`
