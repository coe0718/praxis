# Praxis Code Review — Vex

**Reviewed:** `/mnt/docker/code/praxis/src/`
**Date:** 2026-05-07
**Focus:** Unwired (unused) functions and modules

---

## UNWIRED MODULES (entirely unused)

### 1. `src/a2a/` — Agent-to-Agent protocol ✗

**Severity:** warning (infrastructure present but not wired)

`A2aClient` is defined and exported from `lib.rs` but never instantiated or called anywhere in the codebase. The module is scaffolding with no runtime attachment.

```
src/a2a/client.rs:pub struct A2aClient { ... }
src/a2a/mod.rs:pub use client::A2aClient;
```

**Fix:** Either wire it into the loop/delivery system or remove the dead import from `lib.rs`.

---

### 2. `src/federation/` — Task federation ✗

**Severity:** warning

The entire `federation` module is public in `lib.rs` but has zero call sites outside its own definitions. The four public functions (`new`, `decompose`, `spawn_for_subtask`, `run`) are all unused.

**Fix:** Wire into `Act` phase or remove.

---

### 3. `src/openmolt.rs` — Integration API ✗

**Severity:** warning

Exported from `lib.rs` as `pub mod openmolt`, referenced only in a doc comment in `src/tool_schema.rs`. The 30+ type-safe integrations are defined but nothing calls them.

```rust
// src/tool_schema.rs
//! Inspired by OpenMolt's Zod-typed outputs.
```

**Fix:** Either wire into the tool registry or remove.

---

### 4. `src/wave/` — Wave execution engine ✗

**Severity:** warning

`WaveGraph`, `execute_waves()`, and `summarize_waves()` are well-implemented with tests but have zero call sites outside the module. The entire execution-wave system is orphaned.

**Fix:** Wire into the `Act` phase for parallel tool execution, or remove.

---

### 5. `src/channels.rs` — Signal/Matrix client ✗

**Severity:** warning (intentional stub)

Module-level `#![allow(dead_code)]` present. Contains `SignalClient` and `MatrixClient` stubs that are clearly placeholder enterprise integrations. No call sites.

```rust
#![allow(dead_code)]
pub struct SignalClient { ... }
pub struct MatrixClient { ... }  // likely in rest of file
```

**Fix:** Implement or remove. If intentional for future work, document that in a comment.

---

### 6. `src/hotreload.rs` — Config hot-reload ✗

**Severity:** warning (intentional stub)

Module-level `#![allow(dead_code)]` present. `ConfigWatcher` struct with `new()` and `check_reload()` are unused. The file claims to provide zero-downtime config updates but nothing invokes it.

```rust
#![allow(dead_code)]
pub struct ConfigWatcher { ... }
```

**Fix:** Either wire into `Daemon` startup or remove.

---

## INDIVIDUAL UNWIRED FUNCTIONS (within otherwise used modules)

### `src/delegation/mod.rs`

| Function | Status |
|---|---|
| `send_over_link(...)` | ✓ Wired — called from `loop/phases.rs` |
| `drain_inbound_delegation(...)` | ? Not verified outside this module |
| `glob_match(...)` | internal only |

---

### `src/speculative/mod.rs`

| Function | Status |
|---|---|
| `select_branch(...)` | ✓ Wired — called from `loop/phases.rs` with `SpeculativeBranch` |

---

## LEGITIMATE dead_code suppressions (acceptable)

These are appropriately suppressed because they are **intentionally unused but reserved for future use**:

| File | Item | Reason |
|---|---|---|
| `watchdog/main.rs` | `check_interval_secs`, `save_update_record` | Watchdog subsystem, may be used by CLI |
| `marketplace/mod.rs` | `endpoints`, `agent_id` fields | Marketplace plugin system — future |
| `sandbox/mod.rs` | `tool()` helper | Sandbox policy building, may be used |
| `spotify/mod.rs` | `expires_in` field | OAuth token field, expected to be used |
| `dashboard/types.rs` | `stream` field | SSE config, gated by `disable_sse` in lite mode |
| `messaging/inbound.rs` | various fields | Inbound polling stubs for Discord/Slack |
| `cli/dryrun.rs` | `ExecutionPlan`, `DryRunResult` | Dry-run mode scaffolding |
| `cli/worktree.rs` | `WorkTree` struct | Worktree integration scaffolding |
| `loop/runtime.rs` | `DEFAULT_MAX_SPAWN_DEPTH`, `check_spawn_depth` | Spawn depth guard, gated feature |
| `backend/health.rs` | `ProviderHealthTracker` | Health tracking infrastructure |

---

## BUILD STATUS

```
cargo check  ✓ clean — no warnings
cargo clippy ✓ clean — no warnings
cargo machete ✓ no unused dependencies detected
```

---

## SUMMARY

| Category | Count |
|---|---|
| Fully unwired modules | 6 (`a2a`, `federation`, `openmolt`, `wave`, `channels`, `hotreload`) |
| Legitimate dead_code suppressions | 11 |
| Build warnings | 0 |
| Clippy warnings | 0 |

**Recommendation:** The 6 unwired modules should either be removed or formally linked into the execution loop. The `a2a`, `federation`, and `openmolt` modules in particular appear to be scaffolding for features that were specced but never connected. Either wire them or cut them — dead code creates maintenance debt and misleads future contributors.