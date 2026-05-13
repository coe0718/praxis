# 5-Tier Memory System — Build Spec for Drey

**Context:** this supplements the existing `MemoryStore` trait and `SqliteSessionStore` impl in `src/storage/sqlite/memory.rs`. Don't rebuild what's there — enhance it.

---

## Architecture Overview

```
Session ends → capture.rs: LLM extracts structured memories (one call, full context)
                      ↓
              observer.rs: tokio cron task, writes hot memories to SQLite + FTS5
                      ↓
              reflector.rs: daily consolidation pass → synthesizes hot→cold
                      ↓                         ↓
              search.rs: unified FTS5 × semantic × file search
                      ↓
              loader.rs: inject top-N at session start + recovery check
                      ↓
              decay.rs: TTL pipeline, demotion, expiry
```

All files go in `/mnt/docker/code/praxis-clean/src/memory/`.
All use `rusqlite` (already in deps with `bundled` feature).
Zero external dependencies except `serde_json` and `chrono` (both already in deps).

---

## What Already Exists (do NOT rebuild)

| File | Location | What It Does |
|------|----------|--------------|
| `MemoryStore` trait | `src/memory/types.rs` | All CRUD methods: insert_hot, insert_cold, search, boost, forget, consolidate, decay |
| `MemoryLinkStore` trait | `src/memory/types.rs` | add_memory_link, links_for, linked_memories |
| `MemoryType` enum | `src/memory/types.rs` | Episodic (90d), Semantic (180d), Procedural (365d) with decay_days() |
| `MemoryTier` enum | `src/memory/types.rs` | Hot, Cold |
| `consolidate_memories()` | `src/storage/sqlite/memory_consolidation.rs` | Clusters hot ≥7d by tag (min 3), promotes to cold via `promote_hot_to_cold_atomic`. Prunes dead cold at weight floor×2 |
| `decay_cold_memories()` | `src/storage/sqlite/memory_decay.rs` | ×0.97 multiplier per type's decay_days, floor 0.25 |
| `boost_memory()` | `src/storage/sqlite/memory.rs` | +0.2 importance/weight, updates access_count/last_reinforced |
| `search_memories()` | `src/storage/sqlite/memory.rs` | FTS5 with bm25 ranking, hot+cold combined by score |
| `MemoryLoader` | `src/memory/loader.rs` | Builds query from task/goals, searches, falls back to recent/strongest, expands via links |
| `capture_session_memory()` | `src/loop/reflect.rs` | **STUB** — writes "Session outcome {x} with summary: {y}" at 0.7 importance. NEEDS REPLACEMENT |
| `build_lookup_query()` | `src/memory/query.rs` | Tokenizes task/goal into FTS5 query |
| `to_fts_query()` | `src/memory/query.rs` | Wraps tokens in `"..."` for FTS5 phrase matching |
| Hook system | `src/hooks/mod.rs` | TOML-driven shell hook system. Fire observers/interceptors on events |
| Post-reflect pipeline | `src/loop/runtime.rs` `execute_reflect()` | Already calls `decay_cold_memories()`, `consolidate_memories()` after reflect |

---

## What to Build

### 1. `src/memory/capture.rs` — Post-Session Extraction

**Purpose:** Replace the stub `capture_session_memory()` with intelligent LLM extraction.

**Trigger:** Called at end of reflect phase (modify `capture_session_memory()` in `src/loop/reflect.rs`).

**How it works:**
- Receives: full session context (`SessionState`, `StoredSession`, conversation transcript)
- Constructs a structured extraction prompt for any cheap model (DeepSeek Flash, Haiku, etc.)
- The model returns JSON array of extracted memories:
  ```rust
  #[derive(Deserialize)]
  pub struct ExtractedMemory {
      pub content: String,         // The fact/insight
      pub summary: Option<String>, // One-line summary
      pub importance: f32,         // 0.0-1.0
      pub tags: Vec<String>,       // e.g. ["user-preference", "bug", "architecture"]
      pub memory_type: String,     // "episodic", "semantic", "procedural"
      pub links: Vec<ExtractedLink>,  // connections to existing memories
  }
  
  #[derive(Deserialize)]
  pub struct ExtractedLink {
      pub target_memory_id: i64,
      pub link_type: String,  // "related_to", "contradicts", "caused_by"
  }
  ```
- Calls `self.store.insert_hot_memory()` for each extracted memory with importance >= 0.5
- Creates memory links for any connections found
- Logs extraction stats (memories found, links created, tokens used)

**Prompt should ask the model:**
> From this session, extract durable facts worth remembering. Score each 0-1.
> 0.5+: worth keeping. 0.7+: important insight. 0.9+: irreplaceable knowledge.
> Include: preferences, decisions, bugs found, patterns learned, architecture insights.
> Exclude: transient status, trivial test output, session boilerplate.

**Edge cases:**
- Empty session → skip extraction (no tokens burned)
- Failed LLM call → log warning, fall back to stub behavior
- 0 memories >= 0.5 → don't write anything, that's fine
- Memory already exists (near-identical content) → boost it instead of duplicating

**Dependencies:** `ProviderUsageStore` for tracking extraction cost, `AgentBackend` for LLM call.

**Wire into `reflect()`:**
```rust
// Replace the existing capture_session_memory() call.
// If the LLM extraction succeeds and returns memories, use those.
// If it fails or returns empty, fall back to the current stub.
```

### 2. `src/memory/observer.rs` — Chronological Observation Extractor

**Purpose:** Continuous extraction of durable facts from session transcripts via a tokio cron task. Runs every 15 minutes during active sessions.

**How it works:**
```rust
pub struct MemoryObserver {
    db_path: PathBuf,
    backend: Arc<dyn AgentBackend>,
    store: SqliteSessionStore,
}

impl MemoryObserver {
    pub fn new(paths: &PraxisPaths, backend: Arc<dyn AgentBackend>) -> Self
    
    /// Called by cron or reactive watcher
    pub async fn observe(&self) -> Result<ObservationSummary> {
        // 1. Find sessions ended since last observation run
        // 2. For each session with transcripts not yet observed:
        //    - Read full transcript
        //    - Call LLM to extract observations (same prompt as capture.rs but lighter)
        //    - Insert as hot memories
        // 3. Write checkpoint (last_observed_session_id + timestamp)
        // 4. Return summary
    }
    
    /// Start the cron task (spawned via tokio::spawn)
    pub fn start_cron(self, interval: Duration) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                if let Err(e) = self.observe().await {
                    log::warn!("memory observer: {e}");
                }
            }
        })
    }
}

pub struct ObservationSummary {
    pub sessions_processed: usize,
    pub memories_extracted: usize,
    pub links_created: usize,
}
```

**Checkpoint:** Store in a simple JSON file at `{data_dir}/memory_observer_checkpoint.json`:
```json
{
  "last_session_id": 42,
  "last_observed_at": "2026-05-12T19:00:00Z"
}
```

**Reactive watcher:** For heavy conversation periods, the reflect phase can trigger immediate observation via hook:
```rust
self.events.emit("memory:new_session", &session)?;
```
The observer subscribes to this event and runs extraction immediately instead of waiting for the next cron tick.

**Edge cases:**
- No new sessions → skip (no token burn)
- Already observed → skip (checkpoint tracking)
- Failed extraction on one session → continue to next, log error

### 3. `src/memory/reflector.rs` — Enhanced Daily Consolidation

**Purpose:** Extends the existing `consolidate_memories()` with proper synthesis and contradiction detection. The existing code clusters by shared tag (min 3) — this adds an LLM pass to synthesize the cluster into a coherent cold memory.

**How it works:**
```rust
/// Runs after `consolidate_memories()` picks clusters.
/// For each cluster of 3+ hot memories being promoted:
///   1. Call LLM to synthesize them into one coherent cold memory
///   2. Detect contradictions between new cold and existing cold memories
///   3. Create memory links for detected contradictions
pub fn synthesize_cluster(
    store: &impl MemoryStore + MemoryLinkStore,
    backend: &impl AgentBackend,
    memories: &[StoredMemory],
) -> Result<Option<NewColdMemory>> {
    if memories.len() < 3 {
        return Ok(None);  // Let existing consolidation handle small clusters
    }
    
    // Build prompt: "Synthesize these related memories into one coherent insight..."
    // Model returns: { content, tags, memory_type, contradictions: [existing_memory_ids] }
    
    // Check for contradictions with existing cold memories
    // Use FTS5 search on cold_fts to find potentially conflicting memories
    
    // Create NewColdMemory with source_ids and contradicts fields populated
    // Create MemoryLink::Contradicts for each detected contradiction
}
```

**This REPLACES the existing `promote_hot_to_cold_atomic` for clusters of 3+.**
For clusters of 2 or single memories, the existing path works fine.

**Contradiction surface:** When a contradiction is detected, emit event `memory:contradiction_detected` so it can be surfaced at next session start.

**Edge cases:**
- LLM returns garbage → skip cluster, leave as hot for next consolidation
- Model unavailable → fall back to existing `promote_hot_to_cold_atomic`
- Contradiction with multiple existing colds → link all of them

### 4. `src/memory/search.rs` — Multi-Backend Search

**Purpose:** Unified search across FTS5 + semantic + file-based backends. Wraps the existing `search_memories()` with additional backends.

**How it works:**
```rust
pub struct UnifiedSearch<'a> {
    store: &'a dyn MemoryStore,
    // Can add more search backends later
}

impl<'a> UnifiedSearch<'a> {
    pub fn new(store: &'a dyn MemoryStore) -> Self
    
    /// Run all backends in parallel, merge results by score
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<StoredMemory>> {
        let fts_results = self.store.search_memories(query, limit)?;
        // Future: spawn tokio tasks for semantic + file search
        // Merge: interleave by score, deduplicate by content
        Ok(fts_results)
    }
}
```

For now, this wraps the existing FTS5 search. The multi-backend architecture is pre-built for when vector search gets added.

**Future backends (not building today, just wiring):**
- Semantic: `search_memories_semantic()` already exists but requires LanceDB
- File-based: grep/search across `doc_hashes.json` or LEARNINGS.md

### 5. `src/memory/decay.rs` — Enhanced Decay Pipeline

**Purpose:** Extends the existing `decay_cold_memories()` with hot memory expiry and demotion chain.

**What to build:**
```rust
/// Expire hot memories past their TTL (30 days default)
pub fn expire_hot_memories(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    // DELETE FROM hot_memories WHERE datetime(created_at) <= datetime(?1) 
    // OR (expires_at IS NOT NULL AND datetime(expires_at) <= datetime(?1))
    // Cascade: DELETE FROM hot_fts WHERE rowid IN (...)
}

/// Demote cold memories below floor back to hot before eventual expiry
pub fn demote_cold_to_hot(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    // SELECT cold memories where weight <= COLD_MEMORY_FLOOR (0.25) 
    // AND not reinforced in 90 days
    // INSERT into hot_memories as "archived: {content}" with low importance
    // DELETE from cold_memories + cold_fts
    // This gives them one last chance to be accessed before final expiry
}
```

**Wire into `execute_reflect()`** alongside existing `decay_cold_memories()` and `consolidate_memories()`.

### 6. `src/memory/loader.rs` — Enhanced Session Startup Injection

**Purpose:** Extends the existing `MemoryLoader` to inject full memory context plus contradiction warnings.

**What to change:**
```rust
// In addition to current search/fallback logic:
// 1. Check for unresolved contradictions
pub fn check_contradictions(store: &impl MemoryLinkStore) -> Result<Vec<String>> {
    // Find memories with MemoryLinkType::Contradicts links
    // Return them as formatted warnings for session prompts
}

// 2. Recovery check — catch observations missed during manual resets
pub fn check_missed_observations(store: &impl MemoryStore, checkpoint: &ObservationCheckpoint) -> Result<usize> {
    // Count sessions since last observation checkpoint without extracted memories
    // If gap detected, return count so observer can backfill
}
```

---

## Wire Points Summary

| When | What | Where to Change |
|------|------|-----------------|
| Session ends | `capture.rs`: LLM extraction | `src/loop/reflect.rs` → replace `capture_session_memory()` body |
| Post-reflect (every session) | `decay.rs`: expire hot memories | `src/loop/runtime.rs` → `execute_reflect()` — add `MemoryObserver::observe()` call |
| Post-reflect (daily) | `reflector.rs`: synthesize clusters | `src/storage/sqlite/memory_consolidation.rs` — enhance `consolidate_hot_memories()` |
| Session start | `loader.rs`: contradiction warnings | `src/memory/loader.rs` — add `check_contradictions()` call |
| Every 15 min | `observer.rs`: cron observation | Spawn via `PraxisRuntime` if config enabled |
| Memory link creation | Link contradictions | `reflector.rs` synthesis step |

---

## Schema Changes Needed

None. The existing SQLite tables (`hot_memories`, `cold_memories`, `hot_fts`, `cold_fts`) already have all required columns. The memory link tables (`memory_links`) already support `Contradicts` link type.

---

## Testing Strategy

Each module should be independently testable:

- **capture.rs**: Mock backend returns structured JSON → verify hot memories inserted with correct tags
- **observer.rs**: Create test sessions → run observer → verify memories extracted
- **reflector.rs**: Insert 3+ related hot memories → run synthesis → verify cold memory created with contradictions linked
- **search.rs**: Insert known memories → search → verify results in correct order
- **decay.rs**: Insert expired hot + demoted cold → verify cleanup count
- **loader.rs**: Insert contradictions → verify warning output

---

## Implementation Order

1. `capture.rs` — biggest impact, replaces the stub
2. `loader.rs` enhancement — adds contradiction warnings
3. `decay.rs` — hot expiry + demotion chain
4. `reflector.rs` — LLM synthesis for clusters of 3+
5. `observer.rs` — cron observation (lowest priority, works without it)
6. `search.rs` — multi-backend wrapper (lowest priority, FTS5 works now)
