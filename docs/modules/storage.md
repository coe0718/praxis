# Storage

> SQLite persistence layer for sessions, memories, approvals, quality, and provider usage

## Overview

The `storage` module provides Praxis's complete persistence layer, built on SQLite with WAL mode. It defines a set of traits (`SessionStore`, `ApprovalStore`, `QualityStore`, `ProviderUsageStore`, `OperationalMemoryStore`, `AnatomyStore`, `DecisionReceiptStore`, `SessionSearchStore`) and implements them through a single concrete type: `SqliteSessionStore`.

The trait-based design allows the runtime's generic `PraxisRuntime` to remain decoupled from SQLite specifics, while the concrete implementation consolidates all persistence into a single database file (`data_dir/praxis.db`).

The module is organized as:
- **Trait definitions** (`storage/mod.rs` and submodules) — Pure traits with associated types.
- **SQLite implementation** (`storage/sqlite/`) — Concrete `SqliteSessionStore` with 16+ submodules covering schema, sessions, approvals, memory, quality, providers, decisions, search, and more.

## Architecture

### Store Traits

| Trait | Purpose |
|---|---|
| `SessionStore` | Record and query session history |
| `ApprovalStore` | Queue, list, approve/reject tool approval requests |
| `QualityStore` | Record reviews, eval runs, and quality summaries |
| `ProviderUsageStore` | Track token usage and costs per session/provider/phase |
| `OperationalMemoryStore` | Manage "do not repeat" entries and known bugs |
| `AnatomyStore` | Index file summaries and modification times |
| `DecisionReceiptStore` | Record and query Decide-phase audit receipts |
| `SessionSearchStore` | Full-text search across past session outcomes |

### Key Data Types

| Type | Purpose |
|---|---|
| `SessionRecord` | New session to record: day, timestamps, outcome, goal/task, action summary |
| `StoredSession` | Persisted session with auto-incremented ID and session number |
| `NewApprovalRequest` | Tool approval request: tool name, summary, write paths, payload |
| `StoredApprovalRequest` | Persisted approval with status, timestamps, and optional note |
| `ApprovalStatus` | Lifecycle: `Pending` → `Approved`/`Rejected` → `Claiming` → `Executed` |
| `ReviewRecord` | Reflect-phase review: session, goal, status, findings |
| `EvalRunRecord` | Eval check: session, eval ID, status, severity, summary |
| `EvalStatus` | `Passed`, `Failed`, `Skipped` |
| `EvalSeverity` | `Cosmetic`, `Functional`, `TrustDamaging` |
| `NewDecisionReceipt` | Decide-phase audit: reason code, chosen action, context sources, confidence |
| `OperationalMemoryCounts` | Counts of do-not-repeat entries and known bugs |

### SqliteSessionStore

The concrete implementation wraps a `PathBuf` to the database file. It:
- Opens connections with WAL mode and a 5-second busy timeout.
- Auto-creates parent directories.
- Delegates to submodule functions for each trait method.

### SQLite Submodules

| Submodule | Purpose |
|---|---|
| `schema` / `schema_data` | Table creation and migration |
| `sessions` | Session recording and querying |
| `approvals` | Approval queue management |
| `quality` | Reviews, eval runs, summaries |
| `providers` | Provider usage tracking and token accounting |
| `memory` | Hot/cold memory CRUD |
| `memory_decay` | Cold memory importance decay |
| `memory_consolidation` | Hot-to-cold memory migration |
| `memory_links` | Memory link and contradiction management |
| `anatomy` | File anatomy index |
| `decisions` | Decision receipt storage |
| `ops` | Operational memory (do-not-repeat, known bugs) |
| `opportunities` | Learning opportunity tracking |
| `learning` | Learning run records |
| `search` | Session full-text search |

## Public API

```rust
// Store construction
SqliteSessionStore::new(path: PathBuf) -> Self

// Session management
store.initialize() -> Result<()>
store.record_session(record: &SessionRecord) -> Result<StoredSession>
store.last_session() -> Result<Option<StoredSession>>

// Approvals
store.queue_approval(request: &NewApprovalRequest) -> Result<StoredApprovalRequest>
store.list_approvals(status: Option<ApprovalStatus>) -> Result<Vec<StoredApprovalRequest>>
store.set_approval_status(id, status, note) -> Result<Option<StoredApprovalRequest>>
store.next_approved_request() -> Result<Option<StoredApprovalRequest>>
store.mark_approval_consumed(id) -> Result<()>
store.search_approvals(q, tool, status) -> Result<Vec<StoredApprovalRequest>>

// Quality
store.record_review(record: &ReviewRecord) -> Result<StoredReview>
store.record_eval_run(record: &EvalRunRecord) -> Result<()>
store.latest_eval_summary() -> Result<Option<StoredEvalSummary>>

// Provider usage
store.record_provider_attempts(session_id, attempts) -> Result<()>
store.token_summary_all_time() -> Result<TokenSummaryAllTime>
store.token_usage_by_session(limit) -> Result<Vec<SessionTokenUsage>>
store.token_usage_by_provider() -> Result<Vec<ProviderTokenSummary>>

// Operational memory
store.operational_memory_counts() -> Result<OperationalMemoryCounts>

// Anatomy
store.upsert_anatomy_entry(entry: &NewAnatomyEntry) -> Result<()>
store.anatomy_entry_count() -> Result<i64>

// Search
store.search_sessions(query, limit) -> Result<Vec<SessionSearchResult>>

// Health
store.health_counts() -> Result<(i64, i64, i64)>  // pending, hot, cold
```

## Configuration

| Config | Location | Description |
|---|---|---|
| Database path | `PraxisPaths::database_file` | `data_dir/praxis.db` by default |
| WAL mode | Auto-enabled | `PRAGMA journal_mode=WAL` |
| Busy timeout | Auto-set | `PRAGMA busy_timeout=5000` (5 seconds) |

## Data Files

| File | Purpose |
|---|---|
| `data_dir/praxis.db` | SQLite database (all persistent state) |
| `data_dir/praxis.db-wal` | Write-ahead log (auto-managed by SQLite) |
| `data_dir/praxis.db-shm` | Shared memory index (auto-managed by SQLite) |

## Dependencies

- `rusqlite` — SQLite bindings
- `anatomy` — `NewAnatomyEntry`
- `memory` — Memory types (`NewDoNotRepeat`, `NewKnownBug`, etc.)
- `usage` — `ProviderAttempt`, `ProviderUsageSummary`, token types

## Source

`src/storage/` (trait definitions) and `src/storage/sqlite/` (implementation)
