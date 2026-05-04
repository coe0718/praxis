# Forensics

> Session snapshots and checkpoint recording for replaying, debugging, and auditing agent behavior.

## Overview

The Forensics module captures point-in-time snapshots of the Praxis agent's session state as it progresses through the Orient → Decide → Act → Reflect loop. Each snapshot records the current phase, a checkpoint label, and a full serialized copy of the `SessionState`. This creates a detailed timeline that operators can use to:

- **Debug unexpected behavior** — walk through every state transition a session made.
- **Audit agent decisions** — see what context was available at each decision point.
- **Replay sessions** — trace the exact sequence of phase transitions and tool invocations.

Snapshots are stored in the Praxis SQLite database in a `session_snapshots` table. They are keyed by the session's start timestamp, allowing multiple snapshots per session (one per checkpoint). After a session completes and is assigned a numeric session ID, `attach_session_id` links the snapshots back to the session record.

## Architecture

### `SessionSnapshot` (struct)

| Field | Type | Description |
|---|---|---|
| `session_id` | `Option<i64>` | Linked session row ID (attached after session completion). |
| `started_at` | `String` | RFC 3339 timestamp of when the session began. |
| `phase` | `String` | The agent loop phase at snapshot time (orient/decide/act/reflect). |
| `checkpoint` | `String` | Human-readable checkpoint label (e.g. `"pre_act"`, `"post_reflect"`). |
| `recorded_at` | `String` | RFC 3339 timestamp of when the snapshot was recorded. |
| `state` | `SessionState` | Full serialized session state at the checkpoint. |

### Free functions

- **`record_snapshot(db_path, state, checkpoint)`** — Insert a new snapshot into the database.
- **`attach_session_id(db_path, started_at, session_id)`** — Back-fill the `session_id` column after session creation.
- **`latest_started_at(db_path)`** — Find the most recent session that has snapshots.
- **`load_snapshots(db_path, started_at)`** — Load all snapshots for a given session, ordered chronologically.

## Public API

```rust
// Record a snapshot
record_snapshot(db_path: &Path, state: &SessionState, checkpoint: &str) -> Result<()>

// Link snapshots to a session ID
attach_session_id(db_path: &Path, started_at: DateTime<Utc>, session_id: i64) -> Result<()>

// Query
latest_started_at(db_path: &Path) -> Result<Option<String>>
load_snapshots(db_path: &Path, started_at: &str) -> Result<Vec<SessionSnapshot>>
```

## Configuration

No dedicated configuration fields. The module uses `PraxisPaths.database_file` for the SQLite database path.

## Usage

### CLI commands (`praxis forensics`)

```bash
# Show the latest session's snapshots
praxis forensics latest

# Show snapshots for a specific session (by start timestamp)
praxis forensics session --started-at "2026-05-04T12:00:00+00:00"
```

### In code

```rust
use crate::forensics;

// Record a snapshot before acting
forensics::record_snapshot(&paths.database_file, &state, "pre_act")?;

// After session completion, link snapshots to the session
forensics::attach_session_id(&paths.database_file, state.started_at, session_id)?;
```

## Data Files

| File | Table | Purpose |
|---|---|---|
| `{data_dir}/database.sqlite` | `session_snapshots` | Per-checkpoint session state records. |

The table schema includes columns: `id`, `session_id`, `session_started_at`, `phase`, `checkpoint`, `state_json`, `recorded_at`.

## Dependencies

- **`state`** — `SessionState` is the primary data captured in each snapshot.
- **`storage/sqlite`** — Database access via `rusqlite`.
- **`chrono`** — Timestamp handling.
- Used by: the Praxis runtime loop during phase transitions.

## Source

`src/forensics/mod.rs`
