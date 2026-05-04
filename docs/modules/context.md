# Context

> Token-aware context assembly, compaction, and handoff for the agent loop

## Overview

The `context` module is the gatekeeper for everything the LLM sees during a session. It collects identity files, memories, goals, tool manifests, journal entries, operator model data, and progressive context from the working tree — then fits them into a strict token budget before handing the assembled context to the Orient phase.

Context management in Praxis is designed around a "pressure" model. As sources are loaded into the context window, the budget engine tracks how full the window is. When pressure crosses 50%, a handoff note is written so the next session can resume gracefully. When it reaches 80%, automatic compaction is triggered to start fresh on the next run. An adaptive layer observes which sources correlate with successful sessions and quietly rebalances the budget over time.

Key design goals:
- **Never exceed the token budget** — sources that don't fit are summarized or dropped, never truncated mid-sentence.
- **Preserve anchor information** — goal IDs, boundary rules, dates, and operator-facing keywords are kept even when content is summarized.
- **Avoid redundant file reads** — files read earlier in the same session are replaced with anatomy summaries, saving tokens.
- **Defense in depth** — all context files are scanned for prompt injection patterns before loading.

## Architecture

### Submodules

| Submodule | Purpose |
|---|---|
| `budget` | Token budget allocation engine with priority-based source scheduling |
| `compaction` | Interactive and automatic context window compaction |
| `handoff` | Handoff notes for cross-session continuity |
| `cache` | Persistent context cache for warm-starting Orient |
| `progressive` | Per-directory context file discovery (`.praxis.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules`) |
| `loader` | Top-level context assembly — loads all sources and runs the budgeter |
| `adaptive` | Self-tuning budget weights based on session outcome feedback |
| `files` | Tracked file reader with deduplication and injection scanning |
| `summarize` | Anchor-preserving content summarization |
| `injection` | Prompt injection pattern detection |

### Key Types

- **`ContextBudgeter`** — Stateless allocator. Takes a config and a list of `ContextSourceInput` values, returns a `BudgetedContext` with included/dropped sources and token counts.
- **`BudgetedContext`** — The assembled result. Tracks total budget, tokens used, included sources (each marked as summarized or not), and dropped source names. Exposes `pressure_pct()`, `render()`, and `summary()`.
- **`BudgetedSource`** — A single source within the budget: name, content, token count, and whether it was summarized.
- **`ContextCache`** / `ContextCacheEntry` — Persistent JSON cache (`context_cache.json`) of high-value excerpts from the last Reflect phase. Entries expire after 48 hours.
- **`HandoffNote`** — Written when context pressure exceeds 50%. Records goal, completed items, remaining items, key facts, and do-not-forget items.
- **`CompactionRequest`** — Written to `compaction.json` by the operator (`praxis compact`) or automatically when pressure exceeds 80%. Consumed once on the next Orient.
- **`ProgressiveContext`** — Result of walking from CWD up to the git root, collecting `.praxis.md`, `AGENTS.md`, `CLAUDE.md`, and `.cursorrules` files. Deeper directories override parents.
- **`LocalContextLoader`** — Orchestrates the full load: reads all sources via `TrackedContextReader`, applies adaptive config, and runs the budgeter.
- **`TrackedContextReader`** — Reads files, scans for injection, indexes in the anatomy store, and replaces repeated reads with compact summaries.
- **`AdaptiveState`** — Tracks per-source success/failure rates and computes multipliers (0.8×–1.2×) that reweight the budget.

## Public API

```rust
// Budget allocation
ContextBudgeter.allocate(config: &AppConfig, inputs: Vec<ContextSourceInput>) -> BudgetedContext
BudgetedContext.pressure_pct() -> f32
BudgetedContext.render() -> String
BudgetedContext.summary() -> String

// Compaction
request_compact(data_dir: &Path, req: &CompactionRequest) -> Result<()>
consume_compact(data_dir: &Path) -> Result<Option<CompactionRequest>>
compact_if_needed(data_dir: &Path, pressure_pct: f32, goal: Option<&str>, now: DateTime<Utc>) -> Result<bool>
compaction_pending(data_dir: &Path) -> bool

// Handoff
handoff::write_if_needed(data_dir, pressure_pct, goal, action_summary, now) -> Result<bool>
handoff::load(data_dir) -> Option<HandoffNote>
handoff::clear(data_dir) -> Result<()>

// Context cache
write_context_cache(path, cache) -> Result<()>
load_context_cache(path, now) -> Option<ContextCache>
render_context_cache(cache) -> String

// Progressive context
progressive::load_progressive_context(cwd: &Path) -> Result<ProgressiveContext>

// Adaptive (internal)
adapt_config(config, path) -> Result<AppConfig>
record_context_feedback(path, ended_at, sources, outcome) -> Result<()>
```

## Configuration

Context behavior is controlled by fields in `praxis.toml`:

| Field | Description | Default |
|---|---|---|
| `context.window_tokens` | Total context window size in tokens | Model-dependent |
| `agent.context_ceiling_pct` | Fraction of `window_tokens` usable for context (0.0–1.0) | 0.80 |
| `context.budget` | Array of `{source, priority, max_pct}` entries defining how tokens are allocated | Built-in defaults |

The `context.budget` array is sorted by `priority` (ascending). Each entry reserves up to `max_pct` of the total budget for its source. Sources that exceed their cap are summarized.

## Usage

### CLI

```bash
# Request a full context reset for the next session
praxis compact

# Check context status (via status report)
praxis status
```

### Programmatic

The runtime calls `LocalContextLoader::load()` during Orient, passing the current config, paths, session state, and open goals. The loader returns a `BudgetedContext` that is injected into the Decide and Act prompts.

## Data Files

| File | Purpose |
|---|---|
| `data_dir/context_cache.json` | Persistent context cache from last Reflect |
| `data_dir/handoff_note.json` | Cross-session handoff note (cleared after consumption) |
| `data_dir/compaction.json` | Pending compaction request (cleared after consumption) |
| `data_dir/context_adaptation.json` | Adaptive budget state (success/failure per source) |
| `data_dir/session_state.json` | Live session state including file read records |

## Dependencies

- `config` — `AppConfig`, `ContextSourceConfig`
- `storage` — `AnatomyStore`, `DecisionReceiptStore`, `OperationalMemoryStore`
- `memory` — `MemoryLoader`, `OperationalMemoryLoader`
- `identity` — `Goal`
- `anatomy` — `build_entry`, `render_summary`
- `skills` — `render_catalog`
- `state` — `SessionState`, `FileReadRecord`

## Source

`src/context/`
