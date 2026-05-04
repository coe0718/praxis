# Boundaries

> Hard-limit maintenance and weekly alignment review for the agent's operator

## Overview

The `boundaries` module manages the agent's hard limits — the rules the operator defines to constrain what Praxis can and cannot do. Boundaries are stored as bullet items under a `## Boundaries` heading in the identity file (typically `SOUL.md` or `IDENTITY.md`).

This module also implements a weekly review cycle. Every 7 days, the agent prompts the operator to confirm whether their hard limits have changed. This ensures that boundary drift — where the agent slowly normalizes away from the operator's original constraints — is caught and corrected.

The design is intentionally simple: boundaries are plain text in markdown, not a structured schema. This makes them easy for the operator to edit directly, easy for the agent to understand, and easy to diff in version control.

## Architecture

### Key Types

- **`BoundaryReviewState`** — Persisted state tracking the last review confirmation. Fields: `last_confirmed_at` (RFC 3339 timestamp), `last_note` (optional operator note).

### Key Functions

| Function | Purpose |
|---|---|
| `list_boundaries(path)` | Parse all `- item` entries under `## Boundaries` in the given file |
| `add_boundary(path, rule)` | Append a new rule under the `## Boundaries` heading (creates the section if absent) |
| `confirm_review(path, now, note)` | Record a review confirmation with optional note |
| `review_prompt(state, now)` | Returns a prompt string if a review is due, or `None` |

### Constants

- **`REVIEW_INTERVAL_DAYS = 7`** — Days between required boundary reviews.
- **`BOUNDARIES_HEADING = "## Boundaries"`** — Markdown heading used to locate the section.

## Public API

```rust
// State management
BoundaryReviewState::load_or_default(path: &Path) -> Result<Self>
BoundaryReviewState::save(path: &Path) -> Result<()>
BoundaryReviewState::save_if_missing(path: &Path) -> Result<()>
BoundaryReviewState::review_due(now: DateTime<Utc>) -> bool

// Boundary operations
list_boundaries(path: &Path) -> Result<Vec<String>>
add_boundary(path: &Path, rule: &str) -> Result<()>
confirm_review(path: &Path, now: DateTime<Utc>, note: Option<&str>) -> Result<BoundaryReviewState>
review_prompt(state: &BoundaryReviewState, now: DateTime<Utc>) -> Option<String>
```

## Configuration

| Config | Location | Description |
|---|---|---|
| Boundary rules | `SOUL.md` or `IDENTITY.md` under `## Boundaries` | Markdown bullet items |
| Review state | `data_dir/boundary_review.json` | Last confirmation timestamp |

No `praxis.toml` fields are involved — the 7-day review interval is a fixed constant.

## Usage

### CLI

```bash
# Add a new boundary rule
praxis boundaries add "Never send messages after 22:00 local time"

# List current boundaries
praxis boundaries list

# Confirm a weekly review
praxis boundaries confirm --note "All limits still valid"

# Check if a review is due (used in morning brief)
praxis boundaries status
```

### Automatic

- The **morning brief** includes a `⚠️` warning when a boundary review is overdue.
- The **status report** includes `boundary_review_due` and `last_boundary_review` fields.
- The **Reflect phase** may prompt the agent to check boundary compliance.

## Data Files

| File | Purpose |
|---|---|
| `data_dir/boundary_review.json` | Review state (last confirmed timestamp, optional note) |
| `SOUL.md` / `IDENTITY.md` | Boundary rules live under `## Boundaries` |

## Dependencies

- `chrono` — Timestamp handling
- `serde_json` — Review state serialization

## Source

`src/boundaries.rs`
