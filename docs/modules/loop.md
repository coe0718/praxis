# Loop (Agent Loop Runtime)

> The four-phase Orient → Decide → Act → Reflect session loop that drives every Praxis agent.

## Overview

The loop module is the heart of Praxis. Every time the agent "thinks," it runs through a single session cycle managed by this module. A session begins in the **Orient** phase, where context is assembled from identity files, goals, tools, and memory. It then moves to **Decide**, which selects the next unit of work — a goal, an approved tool request, or an operator-injected task. The **Act** phase executes the chosen action via an LLM backend or a tool invocation. Finally, **Reflect** records the session outcome, runs quality checks, computes scores, captures memories, and proposes self-evolution.

Each phase is isolated behind a method on `PraxisRuntime`, making it straightforward to test, extend, or skip phases. The loop is designed to be idempotent: if a session is interrupted (e.g., by a crash), it can be resumed from the last saved phase boundary via `session_state.json`.

The module also houses two auxiliary subsystems: **steer notes** (mid-run operator nudges injected after tool calls) and **notifications** (background-task completion alerts sent to messaging platforms).

## Architecture

### Core Types

| Type | Description |
|---|---|
| `PraxisRuntime<B,C,E,G,I,S,T>` | The main runtime struct. Generic over backend, clock, events, goal parser, identity policy, store, and tool registry. Owns no data — borrows references for each session. |
| `RunOptions` | Input to `run_once()`: `once` (single pass), `force` (bypass quiet hours), `task` (operator-injected task string). |
| `RunSummary` | Output of `run_once()`: outcome, phase reached, goal/task selected, and action summary. |
| `SessionPhase` | Enum: `Orient`, `Decide`, `Act`, `Reflect`, `Sleep`. Tracked in `SessionState` and persisted across crashes. |
| `GoalDecision` | Result of goal planning: `Selected(Goal)`, `Waiting(String)`, or `Complete`. |

### Phase Flow

```
run_once(options)
  │
  ├─ consume wake intent (urgent bypasses quiet hours)
  ├─ quiet-hours gate → may defer session
  ├─ load_or_create_state (resumes incomplete sessions)
  │
  └─ while phase ≠ Sleep:
       └─ run_phase(state)
            ├─ Orient  → load context, goals, tools, anatomy, delegation queue
            ├─ Decide  → pick task / approved tool / goal via choose_goal()
            ├─ Act     → execute tool request OR finalize_action via LLM
            └─ Reflect → record session, reviewer, evals, score, memory, evolution
```

### Submodules

| File | Purpose |
|---|---|
| `runtime.rs` | `PraxisRuntime` struct, `run_once()`, `execute_reflect()`, `check_inactivity_timeout()`, morning brief dispatch |
| `phases.rs` | `orient()`, `decide()`, `act()` implementations — the core phase logic |
| `reflect.rs` | `reflect()`, `capture_session_memory()`, `maybe_propose_evolution()`, skill synthesis |
| `planner.rs` | `choose_goal()` — dependency-aware, unblocking-preferred goal selection with wake conditions |
| `session.rs` | `validate_options()`, `should_defer_for_quiet_hours()`, `load_or_create_state()`, `emit()` |
| `outcome.rs` | `final_outcome()` and `compose_summary()` — merge initial outcome with review/eval results |
| `steer.rs` | `SteerNote`, `SteerQueue`, `SteerFileStore` — operator mid-run nudges |
| `notifications.rs` | `BackgroundTask`, `NotificationQueue`, `NotificationFileStore` — task completion alerts |

## Public API

### `PraxisRuntime::run_once()`

```rust
pub fn run_once(&self, options: RunOptions) -> Result<RunSummary>
```

Execute a single Orient → Decide → Act → Reflect cycle. Returns a `RunSummary` with the outcome and selected goal/task. This is the only public entry point; the daemon calls it in a loop.

### `check_spawn_depth()`

```rust
pub fn check_spawn_depth(config: &AppConfig, current_depth: u32) -> Result<()>
```

Validates whether the agent is allowed to spawn a sub-agent. Workers are always denied; orchestrators are limited by `max_spawn_depth`.

### Planner: `choose_goal()`

```rust
pub fn choose_goal(goals: &[Goal], data_dir: &Path, now: DateTime<Utc>) -> Result<GoalDecision>
```

Selects the next goal from `GOALS.md`. Prioritizes goals that unblock the most dependents. Respects `blocked_by` dependencies, parent-child ordering (children first), and `wake_when` conditions (`file_exists:`, `env:`, `after:`).

### Steer: `SteerFileStore::push_note()`

```rust
pub fn push_note(path: &Path, text: &str, source: &str) -> Result<SteerNote>
```

Push a steer note to the file-backed queue. Used by CLI and Telegram `/steer` commands.

### Notifications: `NotificationQueue::register()` / `complete()`

```rust
pub fn register(&self, task_id, channel_id, platform, description)
pub fn complete(&self, task_id: &str) -> Option<BackgroundTask>
```

Track background tasks and auto-notify the originating channel on completion.

## Configuration

### `praxis.toml` — `[agent]` section

| Field | Type | Default | Description |
|---|---|---|---|
| `inactivity_timeout_secs` | `Option<u64>` | `None` | Gracefully end session after this many seconds of no tool activity (min 60) |
| `max_session_tool_calls` | `Option<usize>` | `None` | Cap on tool calls per session |
| `disable_sub_agents` | `bool` | `false` | Prevent sub-agent spawning |
| `role` | `string` | `"worker"` | Agent role: `"worker"` or `"orchestrator"` |
| `max_spawn_depth` | `u32` | `0` | Max nesting depth for sub-agent spawning |

### `praxis.toml` — `[runtime]` section

| Field | Type | Default | Description |
|---|---|---|---|
| `quiet_hours_start` | `string` | `"23:00"` | Start of quiet hours (HH:MM, local timezone) |
| `quiet_hours_end` | `string` | `"07:00"` | End of quiet hours |

### `praxis.toml` — `[features]` section

| Flag | Description |
|---|---|
| `speculative_execution` | Enable branching Act phase (generates a conservative alternative plan and picks the higher-scoring branch) |
| `delegation` | Enable agent-to-agent delegation in Act phase |

## Usage

### CLI — run a single session

```bash
praxis run --once
praxis run --once --task "Review open PRs"
praxis run --once --force   # bypass quiet hours
```

### Steer notes

```bash
praxis wake --task "steer:focus on test coverage"
# or via Telegram: /steer focus on test coverage
```

### Scheduled jobs (daemon triggers)

When running as a daemon, cron jobs defined in `scheduled_jobs.json` inject tasks into sessions automatically.

## Data Files

| File | Purpose |
|---|---|
| `session_state.json` | Live session state across phase boundaries; saved after every phase transition |
| `steer_queue.json` | Pending steer notes for mid-run injection |
| `score.jsonl` | Per-session irreplaceability scores |
| `evolution.jsonl` | Append-only self-evolution proposals |
| `SELF_EVOLUTION.md` | Human-readable render of evolution log |
| `evals/examples.jsonl` | Synthetic example training triples |
| `system_anomalies.jsonl` | System snapshot records |
| `brief_sent.txt` | Date of last morning brief (prevents duplicates) |
| `postmortem.md` | Session postmortem records |

## Dependencies

- **state** — `SessionState`, `SessionPhase`
- **config** — `AppConfig` for quiet hours, security, feature flags
- **paths** — `PraxisPaths` for all file locations
- **time** — `Clock` trait, `is_quiet_hours()`
- **heartbeat** — `write_heartbeat()` after every phase
- **hooks** — `HookRunner` for phase/tool interceptors and observers
- **identity** — `GoalParser`, `IdentityPolicy`, `Goal`
- **context** — `LocalContextLoader`, compaction, handoff notes
- **memory** — hot/cold memory stores, links, operational memory
- **storage** — `SessionStore`, `ApprovalStore`, `QualityStore`, and other store traits
- **tools** — `ToolRegistry`, `execute_request()`, `LoopGuard`, `SecurityPolicy`
- **wakeup** — `consume_intent()` for wake-on-intent
- **lite** — `LiteMode` for capability gating
- **quality** — `LocalReviewer`, `LocalEvalSuite`, `EvalRunner`
- **score** — `SessionScore`, `record_score()`
- **evolution** — `EvolutionStore`, `maybe_propose_evolution()`
- **learning** — daily learning runs (gated by lite mode)
- **curator** — autonomous skill grading (gated by lite mode)
- **brief** — morning brief generation (gated by lite mode)
- **events** — `EventSink` for structured event logging
- **forensics** — `record_snapshot()` for forensic snapshots

## Source

`src/loop/` — `mod.rs`, `runtime.rs`, `phases.rs`, `reflect.rs`, `planner.rs`, `session.rs`, `outcome.rs`, `steer.rs`, `notifications.rs`, `tests.rs`, `tool_guard_tests.rs`, `phases_tests.rs`
