# Praxis Status — 2026-05-07

One document covering all gap/feature tracking. Supersedes: GAP_ANALYSIS_HERMES.md, ECOSYSTEM_REVIEW.md, NEEDS_FINISHED.md, GAP_ANALYSIS_HERMES_OPENCLAW.md.

---

## Build Status

```
cargo check ✅ ZERO ERRORS, ZERO WARNINGS
cargo fmt ✅
cargo test ✅ ALL PASS
git status CLEAN (all committed)
```

---

## Gap Scoreboard (62 total)

| State | Count | Meaning |
|--------|-------|---------|
| ✅ Closed | 62 | Fully implemented and wired into runtime |
| 🔴 External | 1 | Blocked on external dependency |

### ✅ Fully Closed (62)

| # | Feature | Implementation |
|---|---------|----------------|
| 1 | Core Agent Loop | `src/loop/runtime.rs` + `src/loop/phases.rs` — Orient/Decide/Act/Reflect |
| 2 | Sessions Spawn | `src/session/spawn.rs` — programmatic session creation for kanban workers |
| 3 | Plugin System | `src/plugins/mod.rs` — dynamic libloading, `should_block` + `rewrite_tool_output` hooks |
| 4 | Skills Hub | `src/skills/mod.rs` — load_catalog, fetch_remote_catalog, install_skill_from_url + CLI |
| 5 | Tool Approval Queue | `src/tools/` — file-read, git-query, shell-exec, web-fetch with required_level |
| 6 | Observability | `src/observability/langfuse.rs` — real Langfuse HTTP client |
| 7 | One-Shot Mode | `src/cli/mod.rs` — tools enabled by default; `-z/--no-tools` for true one-shot |
| 8 | Fallback Chain | `src/cli/fallback.rs` — list/add/remove/reorder/test commands |
| 9 | Webhook System | `src/webhooks.rs` + `src/webhook/` — real signing + delivery |
| 10 | 429 Retry Fallback | `src/backend/retry.rs` — exponential backoff with jitter, configurable policy |
| 11 | Cron Extensions | `src/tools/cron_ext.rs` — no_agent script mode, wake_gate, per-job workdir |
| 12 | Live Canvas | `src/canvas/mod.rs` — streaming HTML workspace surface for dashboard |
| 13 | LanceDB Memory | `src/memory/lance.rs` — vector-backed long-term memory with semantic recall |
| 14 | Auto-Reply | `src/messaging/auto_reply.rs` — rate-limited proactive messaging |
| 15 | i18n | `src/i18n/mod.rs` — 9 languages, builtin + custom translations, `display.language` config |
| 16 | Plugin Marketplace | `src/plugins/marketplace.rs` — remote discovery + install |
| 17 | Inbound Polling | `src/messaging/inbound.rs` — Discord + Slack REST API polling |
| 18 | Vault/Secrets | `src/vault.rs` — AES-GCM encryption, key derivation |
| 19 | Self-Evolution | `src/evolution.rs` — append-only JSONL, proposal lifecycle, approval |
| 20 | Scoring | `src/score.rs` — 4-dimension composite |
| 21 | Memory System | `src/memory/` — hot/cold/link stores + LanceDB vector backend |
| 22 | SQLite Storage | `src/storage/sqlite/` — SessionStore, MemoryStore, ApprovalStore, etc. |
| 23 | Hooks | `src/hooks.rs` — HookRunner with interceptor + observer patterns |
| 24 | Sandbox Isolation | `src/sandbox.rs` — per-channel filesystem isolation policy |
| 25 | Synthetic Evals | `src/examples.rs` — training triples → evals/examples.jsonl |
| 26 | Anatomy | `src/anatomy.rs` — auto-generated CAPABILITIES.md index |
| 27 | Learning | `src/learning.rs` — mines argus report for opportunities |
| 28 | Anomaly Detection | `src/anomaly.rs` — SystemSnapshot → system_anomalies.jsonl |
| 29 | Spotify Integration | `src/spotify/mod.rs` — PKCE OAuth2, 8 actions |
| 30 | Google Meet | `src/meet/mod.rs` — OAuth2 device flow, 4 actions |
| 31 | Provider Registry | `src/providers/mod.rs` — 9+ providers |
| 32 | Dashboard API | `src/dashboard/server.rs` — Axum server with metrics, hooks, SSE |
| 33 | Telegram Brief | `src/messaging/telegram.rs` — morning brief, daily gate |
| 34 | File/Shell/Git/Web Tools | `src/tools/` — all with approval levels |
| 35 | Context Compaction | `src/context/compaction.rs` — context pressure detection + handoff |
| 36 | Morning Brief | `src/brief/` — goal/memory/approval/event aggregation → Telegram |
| 37 | Session State | `src/state.rs` — JSON persist across phase boundaries |
| 38 | Tool Cooldowns | `src/tools/policy.rs` — cooldown enforcement |
| 39 | MCP Integration | `src/mcp/` — discover_mcp_tools wired at daemon startup, MCP server mode |
| 40 | Prometheus Metrics | `src/observability/prometheus.rs` + `/metrics` endpoint |
| 41 | Briefing System | `src/brief/` — 4 aggregation stages |
| 42 | Workspace Init | `src/backend/init.rs` — workspace creation |
| 43 | Identity Files | `src/identity/` — SOUL.md/IDENTITY.md loading |
| 44 | Config Files | `src/config/` — praxis.toml AppConfig parsing |
| 45 | Goals Management | `src/goals.rs` + `src/goals/` — GOALS.md parsing |
| 46 | Argus (Reviewer) | `src/argus/` — per-session reviewer with quality gate |
| 47 | Forensics | `src/forensics/` — event chain replay |
| 48 | CLI Subcommands | `src/cli/` — ask, run, skills, fallback, tui, daemon |
| 49 | DAEMON Mode | `src/daemon.rs` + `src/loop/runtime.rs` — cron-wake loop |
| 50 | TUI Dashboard | `src/tui/` — ratatui full-screen dashboard (feature-gated) |
| 51 | Crypto Utils | `src/crypto/` — HMAC, SHA-256, Ed25519, hex utils |
| 52 | Dashboard SPA Tabs | `frontend/src/pages/Plugins.tsx` + sidebar integration |
| 53 | Pluggable Platforms | `src/messaging/platform.rs` — Platform trait + PlatformRegistry |
| 54 | Dashboard SSE | FileEventSink → events.jsonl; `/events/recent` handler |
| 55 | Dashboard Prometheus | `/metrics` endpoint mounted in server.rs |
| 56 | Kanban Board | `src/kanban/` — SQLite store, CLI, dispatcher, worker pattern, tools |
| 57 | Curator | `src/curator/mod.rs` — run_cycle() wired into execute_reflect |
| 58 | A2A Sync | `src/a2a/` — client implementation for inter-agent communication |
| 59 | Context Group | `src/messaging/context_group.rs` — conversation grouping |
|| 60 | WASM Sandbox | `src/wasm/mod.rs` — wasmtime execution with capabilities (feature-gated) |
|| 61 | OpenAI-compatible API | `src/backend/openai.rs` — full Chat Completions implementation |
|| 62 | ProcessManager Architecture | `src/process_manager.rs` — async message-passing with Worker/Compactor/Corrector |
|| 63 | Agent Federation | `src/federation/mod.rs` — task decomposition, role assignment, session spawning, result synthesis |

### 🔴 External (1)

| # | Feature | Blocker |
|---|---------|---------|
| 37 | Vercel Sandbox | Requires `vercel.com` project + SDK; infrastructure dependency |

---

## Hermes ↔ Praxis Gap Analysis (from GAP_ANALYSIS_HERMES_OPENCLAW.md)

All 10 items from the OpenClaw gap analysis are now closed:

| # | Feature | Status | Implementation |
|---|---------|--------|----------------|
| 1 | Kanban | ✅ | `src/kanban/` — full board with dispatcher + workers |
| 2 | Sessions spawn | ✅ | `src/session/spawn.rs` — programmatic creation |
| 3 | Curator | ✅ | `src/curator/mod.rs` — run_cycle wired |
| 4 | 429 fallback | ✅ | `src/backend/retry.rs` — exponential backoff |
| 5 | Cron extensions | ✅ | `src/tools/cron_ext.rs` — no_agent, wake_gate |
| 6 | Live Canvas | ✅ | `src/canvas/mod.rs` — streaming HTML surface |
| 7 | LanceDB memory | ✅ | `src/memory/lance.rs` — vector semantic recall |
| 8 | Auto-reply | ✅ | `src/messaging/auto_reply.rs` — rate-limited responses |
| 9 | i18n | ✅ | `src/i18n/mod.rs` — 9 languages |
| 10 | Plugin registry | ✅ | `src/plugins/marketplace.rs` — remote discovery |

---

## External Integrations

| Integration | Status | Notes |
|-------------|--------|-------|
| Telegram | ✅ Working | Polling bot, morning brief, daily gate |
| Discord | ✅ In+Out | `discord.rs` outbound + `inbound.rs` REST polling |
| Slack | ✅ In+Out | `slack.rs` outbound + `inbound.rs` REST polling |
| Spotify | ✅ Working | PKCE OAuth2, 8 actions |
| Google Meet | ✅ Working | OAuth2 device flow, 4 actions |
| Langfuse | ✅ Wired | Real HTTP client |
| Prometheus | ✅ Wired | `/metrics` endpoint |
| Vercel | 🔴 Blocked | No vercel.com project/SDK |

---

## Remaining for Competition ($20k prize)

| # | Feature | Status |
|---|---------|--------|
| 1 | **Agent Federation** | ✅ DONE |

---

## File Inventory

```
Root docs:
 STATUS.md ← this file (supersedes all prior gap/feature docs)
 PRAXIS_GAPS.md → superseded, kept for history
 PRAXIS_DESIGN.md → architecture doc (keep)
 README.md → overview (keep)
 CLAUDE.md → dev guide (keep)

src/ module directories (all with mod.rs):
 a2a/ anatomy/ anomaly/ archive/ argus/ attachments/
 backend/ bench/ boundaries/ brief/ bus/ canary/
 canvas/ cli/ config/ context/ crypto/ curator/
 daemon/ dashboard/ delegation/ events/ evolution/ examples/
 forensics/ hands/ heartbeat/ hooks/ i18n/ identity/
 kanban/ learning/ lib/ lite/ loop/ main/ mcp/
 meet/ memory/ merkle/ messaging/ oauth/ observability/
 paths/ plugins/ profiles/ providers/ quality/
 report/ sandbox/ score/ session/ skills/ speculative/
 spotify/ state/ storage/ time/ tools/ tui/
 usage/ vault/ wakeup/ wave/ webhook/ webhooks/
```

---

_Last updated: 2026-05-07 — Drey_