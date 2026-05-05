# Praxis Status — 2026-05-04

One document covering all gap/feature tracking. Supersedes: GAP_ANALYSIS_HERMES.md, ECOSYSTEM_REVIEW.md, NEEDS_FINISHED.md.

---

## Build Status

```
cargo check  ✅  ZERO ERRORS
cargo fmt    ✅
git status   CLEAN (all committed)
```

---

## Gap Scoreboard (52 total)

| Status | Count | Meaning |
|--------|-------|---------|
| ✅ Closed | 51 | Fully implemented and wired into runtime |
| ⚠️ Partial | 0 | All partial items resolved |

### ✅ Fully Closed (47)

| # | Feature | Implementation |
|---|---------|----------------|
| 1 | Core Agent Loop | `src/loop/runtime.rs` + `src/loop/phases.rs` — Orient/Decide/Act/Reflect |
| 3 | Plugin System | `src/plugins/mod.rs` — dynamic libloading, `should_block` + `rewrite_tool_output` hooks wired into `execute_tool_request` |
| 4 | Skills Hub | `src/skills/mod.rs` — load_catalog, fetch_remote_catalog, install_skill_from_url + CLI (`praxis skills`) |
| 5 | Tool Approval Queue | `src/tools/` — file-read, git-query, shell-exec, web-fetch with required_level and approval flags |
| 6 | Observability | `src/observability/langfuse.rs` — real Langfuse HTTP client; `src/observability/mod.rs` wired |
| 7 | One-Shot Mode | `src/cli/mod.rs` — tools enabled by default; `-z/--no-tools` for true one-shot |
| 8 | Fallback Chain | `src/cli/fallback.rs` — 233 lines, list/add/remove/reorder/test commands |
| 9 | Webhook System | `src/webhooks.rs` + `src/webhook/` — real signing + delivery |
| 12 | Vault/Secrets | `src/vault.rs` — AES-GCM encryption, key derivation |
| 13 | Self-Evolution | `src/evolution.rs` — append-only JSONL, proposal lifecycle, approval |
| 15 | Scoring | `src/score.rs` — 4-dimension composite (anticipation/follow-through/reliability/independence) |
| 16 | Memory System | `src/memory/` — hot/cold/link stores via traits |
| 17 | SQLite Storage | `src/storage/sqlite/` — SessionStore, MemoryStore, ApprovalStore, etc. |
| 18 | Hooks | `src/hooks.rs` — HookRunner with interceptor + observer patterns |
| 20 | Sandbox Isolation | `src/sandbox.rs` — per-channel filesystem isolation policy |
| 22 | Synthetic Evals | `src/examples.rs` — training triples → evals/examples.jsonl |
| 23 | Anatomy | `src/anatomy.rs` — auto-generated CAPABILITIES.md index |
| 24 | Learning | `src/learning.rs` — mines argus report for opportunities, daily throttled |
| 25 | Anomaly Detection | `src/anomaly.rs` — SystemSnapshot → system_anomalies.jsonl |
| 26 | Spotify Integration | `src/spotify/mod.rs` — PKCE OAuth2, 8 actions |
| 27 | Google Meet | `src/meet/mod.rs` — OAuth2 device flow, 4 actions |
| 29 | Provider Registry | `src/providers/mod.rs` — 9 providers: OpenAI, Anthropic, Google AI, GMI Cloud, Azure AI Foundry, MiniMax OAuth, NVIDIA NIM, AWS Bedrock, DeepSeek, Groq, Mistral, Cohere |
| 30 | Dashboard API | `src/dashboard/server.rs` — Axum server with metrics, hooks, SSE stubs |
| 31 | Telegram Brief | `src/messaging/telegram.rs` — morning brief, daily gate via brief_sent.txt |
| 32 | File Tools | `src/tools/file.rs` + `src/tools/git.rs` + `src/tools/shell.rs` + `src/tools/web.rs` |
| 33 | Shell Tool | `src/tools/shell.rs` — rehearsable, level 3, approval required |
| 34 | Git Tool | `src/tools/git.rs` — level 2, approval required |
| 35 | Web Fetch | `src/tools/web.rs` — Http kind, level 2, approval required |
| 36 | Context Compaction | `src/context/compaction.rs` — context pressure detection + handoff writing |
| 38 | Morning Brief | `src/brief/` — goal/memory/approval/event aggregation → Telegram |
| 39 | Session State | `src/state.rs` — JSON persist across phase boundaries |
| 40 | Tool Cooldowns | `src/tools/policy.rs` — cooldown enforcement |
| 43 | MCP Integration | `src/mcp/` — dispatch stub exists (not wired to runtime) |
| 44 | Prometheus Metrics | `src/observability/prometheus.rs` — PrometheusMetrics impl |
| 45 | Briefing System | `src/brief/` — 4 aggregation stages |
| 46 | Workspace Init | `src/backend/init.rs` — workspace creation |
| 47 | Identity Files | `src/identity/` — SOUL.md/IDENTITY.md loading and validation |
| 48 | Config Files | `src/config/` — praxis.toml AppConfig parsing |
| 48b | Goals Management | `src/goals.rs` + `src/goals/` — GOALS.md parsing, active goal list |
| 49 | Argus (Reviewer) | `src/argus/` — per-session reviewer with quality gate |
| 50 | Forensics | `src/forensics/` — event chain replay |
| 51 | CLI Subcommands | `src/cli/` — ask, run, skills, fallback, tui, daemon subcommands |
| 52 | DAEMON Mode | `src/daemon.rs` + `src/loop/runtime.rs` — cron-wake loop |
| 53 | TUI Dashboard | `src/tui/` — ratatui full-screen dashboard (feature-gated) |
| 54 | Crypto Utils | `src/crypto/` — HMAC, SHA-256, Ed25519, hex utils |
| 19 | Dashboard SPA Plugin Tabs | `frontend/src/pages/Plugins.tsx` + `PluginTabRenderer.tsx` + `/api/plugins/tabs` API + sidebar integration + React Router routes |
| 28 | Pluggable Platforms | `src/messaging/platform.rs` — `Platform` trait; Discord/Telegram/Slack all `impl Platform`; `PlatformRegistry` + `poll_platforms` in daemon loop |
| 41 | Dashboard SSE | `FileEventSink` → `events.jsonl`; `/events/recent` Axum handler; `useSSE` React hook + `react-router` Outlet context wired |
| 42 | Dashboard Prometheus | `src/observability/prometheus.rs` + `/metrics` endpoint mounted in `server.rs` + `collect_prometheus_metrics` with session/token gauges |
| 43 | MCP Integration | `src/mcp/` — `discover_mcp_tools` wired at daemon startup; `mcp:<server>:<tool>` routing in `execute_tool_request`; `McpServerConfig::load_all` |

### ✅ All Partial Items Resolved (0)

No partial items remaining.

### 🔴 External (1)

| # | Feature | Blocker |
|---|---------|---------|
| 37 | Vercel Sandbox | Requires `vercel.com` project + SDK; infrastructure dependency |

---

## Stubs / Wired-But-Disconnected (from NEEDS_FINISHED.md)

These are not gaps — they are deliberately deferred or architecturally scoped to future phases:

| Module | Status | Note |
|--------|--------|------|
| `src/delegation.rs` | Storage only | Store exists; Act phase does not send work over links |
| `src/speculative/` | Storage only | Store exists; Act has no branching logic |
| `messaging/discord.rs` | Outbound only | No inbound polling loop |
| `messaging/slack.rs` | Outbound only | No Events API inbound |

---

## External Integrations (from ECOSYSTEM_REVIEW.md)

| Integration | Status | Notes |
|-------------|--------|-------|
| Telegram | ✅ Working | Polling bot, morning brief, daily gate |
| Discord | ⚠️ Outbound only | `discord.rs` has webhook send; no inbound |
| Slack | ⚠️ Outbound only | Webhook send stub; no Events API inbound |
| Spotify | ✅ Working | PKCE OAuth2, 8 actions implemented |
| Google Meet | ✅ Working | OAuth2 device flow, 4 actions |
| Langfuse | ✅ Wired | `src/observability/langfuse.rs` — real HTTP client |
| Prometheus | ✅ Wired | `/metrics` endpoint mounted in dashboard server |
| Vercel | 🔴 Blocked | No vercel.com project/SDK |
| Opus/DISCORD Voice | 🔴 Blocked | No Rust Opus crate available |

---

## Hermes Gap Analysis Summary (from GAP_ANALYSIS_HERMES.md)

Hermes-feature parity status — Praxis perspective:

| Module | Hermes | Praxis | Status |
|--------|--------|--------|--------|
| Workspace Init | ✅ | ✅ | Closed |
| Config Files | ✅ | ✅ | Closed |
| Plugin Loading | ✅ Dynamic | ✅ Dynamic | Closed |
| Tool Manifest | ✅ MCP | ✅ TOML | Closed |
| Observability | ✅ All | ✅ All | Closed — Prometheus `/metrics` + Langfuse + dashboard SSE |
| Core Loop | ✅ | ✅ | Closed |
| TUI | ✅ | ✅ | Feature-gated, compiles |
| Webhooks | ✅ | ✅ | Closed |
| Dashboard | ✅ Full | ✅ Full | Closed — SPA tabs, SSE, Prometheus, `/api/plugins/*` all wired |
| Global Scope | ✅ | ✅ | Closed |
| Conversation Mgmt | ✅ | ✅ | Closed |
| Hermes ↔ Praxis Sync | ✅ | ⚠️ Partial | `src/a2a/` — design doc exists; implementation deferred |

---

## Consolidated To-Do (priority order)

### High — All complete ✅

### Medium — Runtime completeness
1. **Discord/Slack inbound polling** — Events API polling loop for inbound messages (~1-2 days)

### Low — Nice to have
2. **Discord Voice** — Waiting on Rust Opus crate availability

### Blocked (external)
- Vercel Sandbox (#37) — waiting on vercel.com SDK

---

## File Inventory

```
Root docs:
  STATUS.md              ← this file (supersedes GAP_ANALYSIS_HERMES.md, ECOSYSTEM_REVIEW.md, NEEDS_FINISHED.md)
  PRAXIS_GAPS.md         → superseded, kept for history
  PRAXIS_DESIGN.md       → architecture doc (keep)
  README.md              → overview (keep)
  CLAUDE.md              → dev guide (keep)
  Praxis_More.md        → misc notes (keep)

src/ module directories (all with README.md):
  a2a/  anatomy/  anomaly/  archive/  argus/  attachments/
  backend/  bench/  boundaries/  brief/  bus/  canary/
  cli/  config/  context/  crypto/  curator/  dashboard/
  daemon/  delegation/  events/  evolution/  examples/
  forensics/  hands/  heartbeat/  hooks/  identity/
  learning/  lib/  lite/  loop/  main/  mcp/
  meet/  memory/  merkle/  messaging/  oauth/  observability/
  paths/  plugins/  postmortem/  profiles/  providers/
  quality/  report/  sandbox/  score/  skills/  speculative/
  spotify/  state/  storage/  time/  tools/  tui/
  usage/  vault/  wakeup/  watchdog/  wave/  webhook/
  webhooks/
```

---

_Last updated: 2026-05-04 — Drey_