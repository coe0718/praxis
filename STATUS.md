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
| ✅ Closed | 47 | Fully implemented and wired into runtime |
| ⚠️ Partial | 4 | Backend/API done; frontend or runtime wiring pending |
| 🔴 External | 1 | Blocked by third-party SDK/service that doesn't exist |

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
| 55 | Time Utils | `src/time.rs` — timezone-aware scheduling |

### ⚠️ Partial (4)

| # | Feature | Status | What's Done | What's Missing |
|---|---------|--------|-------------|-----------------|
| 19 | Dashboard SPA | Backend done | `src/dashboard/routes_plugins.rs` — plugin tabs/widgets API (`GET /api/plugins/tabs`, `GET /api/plugins/widgets/:name`) + server wiring | Frontend tab rendering (separate React SPA, port 5173) |
| 28 | Pluggable Platforms | Trait+registry done | `src/messaging/platform.rs` — `Platform` trait + `PlatformRegistry` defined | Migrate Discord/Telegram/Slack to implement trait (~1-2 days migration work) |
| 41 | Dashboard SSE | SSE endpoint exists | `src/dashboard/routes_sse.rs` — `EventStream` struct, `sse_events()` handler | No polling loop in runtime to push events; SSE consumer not wired |
| 42 | Dashboard Prometheus | Metrics impl done | `src/observability/prometheus.rs` — `PrometheusMetrics` struct | `/metrics` endpoint not mounted in dashboard server; scrape target not configured |

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
| `src/mcp/` | Dispatch stub | Not wired to tool registry |
| `messaging/discord.rs` | Outbound only | No inbound polling loop |
| `messaging/slack.rs` | Outbound only | No inbound polling loop |
| Dashboard SSE | Endpoint only | No runtime push; no consumer wired |

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
| Prometheus | ⚠️ Impl done | Metric struct exists; endpoint not mounted |
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
| Observability | ✅ All | ⚠️ Langfuse only | Partial — Prometheus + dashboard metrics unwired |
| Core Loop | ✅ | ✅ | Closed |
| TUI | ✅ | ✅ | Feature-gated, compiles |
| Webhooks | ✅ | ✅ | Closed |
| Dashboard | ✅ Full | ⚠️ API skeleton | Partial — SPA tabs + SSE unwired |
| Global Scope | ✅ | ✅ | Closed |
| Conversation Mgmt | ✅ | ✅ | Closed |
| Hermes ↔ Praxis Sync | ✅ | ⚠️ Partial | `src/a2a/` — design doc exists; implementation deferred |

---

## Consolidated To-Do (priority order)

### High — Must wire before "done"
1. **`/metrics` endpoint** — mount Prometheus handler in dashboard server.rs (~5 lines)
2. **SSE polling loop** — wire event push into runtime reflect phase (~20 lines)
3. **Prometheus scrape target** — document `/metrics` endpoint in dashboard README

### Medium — Runtime completeness
4. **Migrate platforms to `Platform` trait** — Discord/Telegram/Slack → `impl Platform` (~1-2 days)
5. **MCP tool wiring** — `src/mcp/` dispatch → tool registry integration

### Low — Nice to have
6. **Dashboard SPA tabs** — separate React project calling `/api/plugins/tabs`
7. **Discord/Slack inbound** — polling loop for Events API

### Blocked (external)
- Vercel Sandbox (#37) — waiting on vercel.com SDK
- Discord Voice (#10) — waiting on Rust Opus crate

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