# Memory

> Multi-tier memory system with hot/cold storage, relational links, operational memory, and persistent user memory.

## Overview

The `memory` module is Praxis's knowledge retention layer. It implements a two-tier hot/cold memory model inspired by human memory: hot memories are recent, high-importance items that decay quickly, while cold memories are consolidated, long-lived knowledge. Memories are typed (episodic, semantic, procedural) with different decay rates, and connected through typed relational links (`caused_by`, `contradicts`, `user_preference`, etc.).

Beyond hot/cold storage, the module also provides **operational memory** (do-not-repeat warnings and known-bug records that influence session behavior) and **user memory** (a persistent key-value store for operator preferences that survives across all sessions).

Memory retrieval uses either SQLite FTS5 full-text search or hybrid semantic search with stored vector embeddings. The `MemoryLoader` orchestrates query construction from the current task or active goals, then expands results through relational links for richer context.

## Architecture

### Core Types (`types.rs`)

| Type | Description |
|------|-------------|
| `MemoryTier` | `Hot` or `Cold` — determines storage and decay behavior. |
| `MemoryType` | `Episodic` (90-day decay), `Semantic` (180-day), `Procedural` (365-day). |
| `NewHotMemory` | Input for creating a hot memory: content, summary, importance, tags, expiration, type. |
| `NewColdMemory` | Input for creating a cold memory: content, weight, tags, source IDs, contradictions, type. |
| `StoredMemory` | A persisted memory from either tier, with ID, score, tags, and type. |
| `MemoryLinkType` | Typed relationship: `CausedBy`, `RelatedTo`, `Contradicts`, `UserPreference`, `FollowUp`. |
| `MemoryLink` | A directed link between two memories with a type. |
| `ConsolidationSummary` | Counts of memories consolidated and pruned in a single pass. |

### Traits

| Trait | Methods |
|-------|---------|
| `MemoryStore` | `insert_hot_memory`, `insert_cold_memory`, `recent_hot_memories`, `strongest_cold_memories`, `search_memories`, `search_memories_semantic`, `decay_cold_memories`, `consolidate_memories`, `get_memory`, `boost_memory`, `forget_memory` |
| `MemoryLinkStore` | `add_memory_link`, `links_for`, `linked_memories` |

### Memory Loading (`loader.rs`)

`MemoryLoader` builds a `LoadedMemoryContext` containing hot, cold, and linked memories. It queries based on the current task or open goals, falls back to recent/strongest when no matches are found, then expands the top hot memories through their relational links.

### Query Construction (`query.rs`)

- `build_lookup_query()` — derives a search query from the requested task (priority) or the first two open goal titles.
- `to_fts_query()` — converts free text into a safe SQLite FTS5 phrase query.

### Operational Memory (`ops.rs`)

| Type | Description |
|------|-------------|
| `NewDoNotRepeat` / `StoredDoNotRepeat` | Warnings about actions that should not be repeated, with severity levels. |
| `NewKnownBug` / `StoredKnownBug` | Known bug signatures with symptoms and fix summaries. |
| `LoadedOperationalContext` | Bundles do-not-repeat warnings and known bugs for the current session. |
| `OperationalMemoryLoader` | Loads operational context based on the current task or goals. |

### User Memory (`user.rs`)

| Type | Description |
|------|-------------|
| `UserMemoryEntry` | A single key-value entry with tags and timestamps. |
| `UserMemory` | Collection of entries, persisted as `user_memory.json`. |

User memory supports `upsert`, `forget`, `search` (substring on key/value/tags), `get`, `keys`, and `render` (markdown for context injection). The `execute_user_memory_action()` function dispatches tool-call actions: `upsert`, `search`, `forget`, `list`.

### Supporting Modules

| Module | Description |
|--------|-------------|
| `vector.rs` | Embedding generation and cosine similarity for semantic search. Currently uses a deterministic hash-based placeholder (128-dim). |
| `conflicts.rs` | Generates `MEMORY_CONFLICTS.md` from all `contradicts` links for operator review. |

## Public API

```rust
// Types
pub use types::{
    ConsolidationSummary, MemoryLink, MemoryLinkStore, MemoryLinkType, MemoryStore,
    MemoryTier, MemoryType, NewColdMemory, NewHotMemory, StoredMemory,
};

// Loading
pub use loader::{LoadedMemoryContext, MemoryLoader};

// Operational
pub use ops::{
    LoadedOperationalContext, NewDoNotRepeat, NewKnownBug,
    OperationalMemoryLoader, StoredDoNotRepeat, StoredKnownBug,
};

// User
pub use user::{UserMemory, UserMemoryEntry};
```

## Configuration

Memory behavior is configured indirectly through `praxis.toml` runtime settings. No module-specific config section exists; decay thresholds are hardcoded per `MemoryType`.

## Usage

```bash
# Store a preference
praxis memory upsert --key "preferred_editor" --value "neovim" --tags "tools,dev"

# Search memories
praxis memory search "deploy"

# List all user memory keys
praxis memory list

# Forget a specific entry
praxis memory forget --key "temp_note"
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `user_memory.json` | JSON | Persistent operator key-value memory. |
| `MEMORY_CONFLICTS.md` | Markdown | Human-readable list of contradicting memory pairs (regenerated). |
| SQLite tables | DB | Hot memories, cold memories, memory links, operational memory (do-not-repeat, known bugs). |

## Dependencies

- `identity` — `Goal` type for query construction
- `storage` — `SqliteSessionStore` implements all memory traits
- `chrono`, `serde`, `anyhow`

## Source

`src/memory/` — `mod.rs`, `types.rs`, `loader.rs`, `query.rs`, `ops.rs`, `user.rs`, `vector.rs`, `conflicts.rs`
