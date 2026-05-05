# Praxis Gap Analysis: Hermes + OpenClaw → Praxis

**Date:** 2026-05-05
**Analyst:** Drey

---

## Purpose

Compare Praxis against Hermes Agent (Python, NousResearch) and OpenClaw (TypeScript/Node.js) to identify features that belong in Praxis. Two separate competitive sources — Hermes because it's our upstream reference, OpenClaw because it's a separate ecosystem worth borrowing from.

---

## Hermes → Praxis

Hermes recent commits (2026-05-04/05) added significant new systems.

### 1. Kanban System (CRITICAL — add to Praxis)

**Location:** `plugins/kanban/`, `tools/kanban_tools.py`

Full dispatcher/worker task board. Tools only registered when `HERMES_KANBAN_TASK` env var is set — normal chat sessions see zero kanban tools.

| Tool | Purpose |
|------|---------|
| `kanban_show` | Read task state: row, parents, children, comments, run history, events |
| `kanban_complete` | Mark task done with structured handoff + metadata |
| `kanban_block` | Transition task to blocked with human-readable reason |
| `kanban_heartbeat` | Signal liveness during long operations |
| `kanban_comment` | Append durable note to task thread |
| `kanban_create` | Create child task (orchestrator fans out work) |
| `kanban_link` | Link tasks into pipeline |
| `kanban_take` | Claim a task |
| `kanban_tasks` | List available tasks |

**Hallucination Gate** (commit #20232): Validates worker-created card claims before they hit the board — prevents a buggy or prompt-injected worker from corrupting sibling/cross-tenant runs.

**Systemd integration:** `plugins/kanban/systemd/hermes-kanban-dispatcher.service`

**Gap for Praxis:** Praxis has no task board. The approval queue is one-way and primitive. A proper Kanban with dispatcher/worker pattern would transform how multi-step work gets done.

**Priority:** HIGH

---

### 2. Curator — Background Skill Maintenance (HIGH — add to Praxis)

**Location:** `agent/curator.py`

Inactivity-triggered background skill maintenance orchestrator (no cron daemon — runs when agent is idle and `interval_hours` has passed).

Key behaviors:
- Auto-transitions agent-created skills: `active → stale (30d) → archived (90d)`
- Pinned skills bypass all transitions
- Never auto-deletes (only archives — archives are recoverable)
- Spawns a forked AIAgent that can pin/archive/consolidate/patch skills
- Creates **umbrella skills** from prefix clusters (e.g. merge `rust-debugging-1`, `rust-debugging-2` into single `rust-debugging` umbrella)
- Writes reports to `logs/curator/{timestamp}/run.json` + `REPORT.md`

**Configuration:**
```python
DEFAULT_INTERVAL_HOURS = 24 * 7   # 7 days
DEFAULT_MIN_IDLE_HOURS = 2
DEFAULT_STALE_AFTER_DAYS = 30
DEFAULT_ARCHIVE_AFTER_DAYS = 90
```

**Gap for Praxis:** Praxis has `src/skills/mod.rs` with `install_skill_from_url` but no automatic curation. Skills accumulate indefinitely. The curator's consolidation strategy (umbrella skills from prefix clusters, class-level instructions over narrow bug-fix skills) is the right approach.

**Priority:** HIGH

---

### 3. Cron Scheduler — Full Expressions + Context Chaining (MEDIUM — already have some)

**Location:** `cron/jobs.py`, `cron/scheduler.py`, `tools/cronjob_tools.py`

Praxis already has `src/daemon.rs` with cron-wake loop. But Hermes has:

- **Schedule types:** one-time duration (`30m`, `2h`, `1d`), timestamp (`2026-02-03T14:00`), interval (`every 30m`), cron (`0 9 * * *`)
- **`context_from`:** chain job outputs from previous runs as context
- **`no_agent` script mode:** run shell scripts directly, no LLM involvement
- **`workdir` per-job:** each job runs from a specific directory
- **Grace period handling:** compute `_grace_seconds` from half the schedule period
- **`wake_gate`:** script output can signal `wakeAgent=false` to skip agent run
- **Lock file:** `~/.hermes/cron/.tick.lock` with thread-safe locking (Unix + Windows)
- **Delivery platforms:** `telegram`, `discord`, `slack`, `whatsapp`, `signal`, `matrix`, `mattermost`, `homeassistant`, `dingtalk`, `feishu`, `wecom`, `weixin`, `sms`, `email`, `webhook`, `bluebubbles`, `qqbot`, `yuanbao`

**Gap for Praxis:** Praxis has basic cron-wake but lacks `context_from`, `no_agent` script mode, `workdir` per-job, and wake gate. The daemon loop could be extended with these.

**Priority:** MEDIUM (incremental improvement)

---

### 4. i18n — Static Message Translation (LOW — nice to have)

**Location:** `agent/i18n.py`, commit #20231

`display.language` config for static message translation (zh/ja/de/es).

**Gap for Praxis:** No i18n. Low priority unless Praxis targets non-English users.

**Priority:** LOW

---

### 5. Other Notable Hermes Fixes (backport to Archiview)

| Fix | Relevance |
|-----|-----------|
| `fix(aux): trigger fallback on 429` | Archiview uses sequential MiniMax calls — rate-limit fallback needed |
| `fix(acp): preserve assistant reasoning metadata` | Session persistence keeps reasoning traces — better for archival quality |
| `fix(openrouter): canonical X-Title` | Proper attribution header |
| `fix(session): serialize JSONL under existing lock` | Thread-safe appends |
| `refactor(env): shared Hermes dotenv loader` | Unified env loading |

---

## OpenClaw → Praxis

OpenClaw is a Node.js personal AI assistant (TypeScript). Separate ecosystem — borrow patterns, don't copy code.

### 6. Live Canvas / Agent Workspace (HIGH — consider for Praxis)

**OpenClaw:** Agent-driven visual workspace. A2UI protocol drives a Canvas surface the operator can see and interact with.

**Praxis gap:** Praxis has no visual canvas. The dashboard is data-only. A live Canvas would give the operator visual feedback on what Praxis is working on.

**Note:** This is architecturally significant. For Praxis, a lighter approach might be a streaming HTML surface in the dashboard that updates as the agent works — without building a full Canvas protocol.

**Priority:** MEDIUM

---

### 7. Sessions Tools — `sessions_spawn` + `sessions_list` (HIGH — add to Praxis)

**OpenClaw:** `sessions_list`, `sessions_history`, `sessions_send`, `sessions_spawn`

**Praxis:** Has session storage (`SqliteSessionStore`) and session state (`src/state.rs`) but no `sessions_spawn` equivalent — the ability to programmatically create a new session and send it work.

**Gap for Praxis:** The Kanban system needs `sessions_spawn` to dispatch workers. A dispatcher pattern requires creating new sessions for each worker task.

**Priority:** HIGH (required for Kanban)

---

### 8. Multi-Agent Routing (MEDIUM — future consideration)

**OpenClaw:** Route inbound channels/accounts/peers to isolated agents. Multiple agents on the same gateway, each with its own SOUL.md.

**Praxis:** Single-agent by design. `src/delegation.rs` exists but Act phase doesn't send work over delegation links. This is a future phase feature.

**Priority:** MEDIUM (deferred)

---

### 9. Memory Plugins — LanceDB with Auto-Recall (MEDIUM)

**OpenClaw:** `memory-lancedb` plugin with auto-recall/capture. Slot-based — one memory plugin active at a time.

**Praxis:** Has `src/memory/` with hot/cold/link stores via traits. `SqliteSessionStore` is the concrete type. No LanceDB integration.

**Gap for Praxis:** A proper vector-backed long-term memory store (LanceDB or pgvector) with semantic recall would significantly improve cross-session continuity.

**Priority:** MEDIUM

---

### 10. Auto-Reply / Proactive Messaging (MEDIUM)

**OpenClaw:** `src/auto-reply/` — proactive auto-reply capability.

**Praxis:** Reactive only (waits for operator or cron to trigger). No proactive messaging.

**Gap for Praxis:** Could work with the Platform trait — if Discord/Slack inbound polling is built, auto-reply is a natural extension.

**Priority:** MEDIUM (requires inbound polling first)

---

### 11. Node System — Companion Apps (LOW)

**OpenClaw:** macOS menu bar app, iOS/Android companion nodes that pair over WebSocket.

**Praxis:** No native companion apps. Dashboard SPA is the closest equivalent.

**Priority:** LOW

---

### 12. Plugin System Architecture (MEDIUM — reference)

**OpenClaw's plugin model:**
- **Bundle plugins:** Codex/Claude/Cursor-compatible layout (`.codex-plugin/`, `.claude-plugin/`)
- **Native plugins:** TypeScript `openclaw.plugin.json` + runtime module
- **Plugin SDK:** Full TypeScript SDK (`plugin-sdk/`)
- **Slot system:** Exclusive slots for memory, context engine
- **ClawHub:** Plugin registry with security review and provenance

**Praxis:** Already has `src/plugins/mod.rs` with `libloading` dynamic loading and hook system (`should_block`, `rewrite_tool_output`). Praxis's plugin model is Rust `.so`-based, which is lower-level but more isolated than OpenClaw's JS plugins.

**Gap for Praxis:** Better plugin discovery and a plugin registry (equivalent to ClawHub) would help. But the core hook system is sound.

**Priority:** LOW (architecture is already good)

---

## Summary: What to Add to Praxis

| # | Feature | Source | Priority |
|---|---------|--------|----------|
| 1 | **Kanban system** (dispatcher/worker/task board) | Hermes | HIGH |
| 2 | **Sessions spawn** (`sessions_spawn`) | OpenClaw | HIGH |
| 3 | **Curator** (skill archiver/consolidator) | Hermes | HIGH |
| 4 | **429 fallback** in auxiliary client | Hermes fix | MEDIUM (Archiview) |
| 5 | **Cron extensions** (`context_from`, `no_agent`, `workdir`, `wake_gate`) | Hermes | MEDIUM |
| 6 | **Live Canvas / visual workspace** | OpenClaw | MEDIUM |
| 7 | **LanceDB memory** with auto-recall | OpenClaw | MEDIUM |
| 8 | **Auto-reply** (requires inbound polling first) | OpenClaw | MEDIUM |
| 9 | **i18n** (display.language) | Hermes | LOW |
| 10 | **Plugin registry/discovery** | OpenClaw | LOW |

---

## Items Already in Praxis (verify parity)

| Feature | Praxis | Notes |
|---------|--------|-------|
| Tool manifest TOML | ✅ `src/tools/` | Hermes uses MCP tools + TOML manifests |
| Hook system | ✅ `src/hooks.rs` | Hermes has `HookRunner` interceptor + observer |
| Skills hub | ✅ `src/skills/mod.rs` | Hermes skills sync from agentskills.io |
| Platform trait | ✅ `src/messaging/platform.rs` | Hermes has `platforms/` plugin |
| Dashboard SPA | ✅ `frontend/` React/Vite | OpenClaw has `canvas-host` |
| MCP integration | ✅ `src/mcp/` | Both have MCP support |
| Observability | ✅ Langfuse + Prometheus | Hermes has Langfuse + custom |
| Evolution/proposals | ✅ `src/evolution.rs` | Hermes has curator for skill evolution |
| Cron/daemon mode | ✅ `src/daemon.rs` | Hermes has `cron/` scheduler |

---

_Last updated: 2026-05-05 — Drey_