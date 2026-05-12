## Shelldex Features (28+ features in implementation pipeline)

### Tier 1 (High Priority - 7 features)
1. Agent-as-Worker Marketplace (CashClaw) ✅ EXISTS in `src/marketplace/`
2. Git-Native Agent Lifecycle (Gitclaw) ✅ DONE — version-controlled identity/rules/memory
3. Zero-LLM Rule-Based Mode ✅ DONE
4. Browser-Only PWA Mode (OpenBrowserClaw) ✅ DONE — WASM agent execution
5. Heartware Personality/Relationship System (TinyClaw, Clawra) ✅ EXISTS in `src/personality.rs`
6. Docker Isolation Mode (IronClaw) ✅ DONE — per-tool container isolation
7. Multi-Process Architecture ✅ ALREADY EXISTS via ProcessManager

### Tier 2 (Medium Priority - 11 features)
- Proactive Agent Mode ✅ EXISTS (src/proactive.rs, src/wakeup/proactive.rs)
- Code-First Integration API (OpenMolt) ✅ DONE — 30+ type-safe integrations
- Signed WASM Plugins (Carapace) ✅ DONE — cryptographic verification for skills
- Scheduled Event Triggers ✅ DONE — cron evaluation, composite conditions, trigger chains
- 32 Built-in Tools (ZeptoClaw) ✅ DONE — 32 tools across 6 categories
- Chinese Platform Channels ✅ DONE (QQ, Feishu, DingTalk, WeChat, WeCom)
- Chinese LLM Providers ✅ N/A — integrated via provider system
- Runtime Skill Creation ✅ DONE — dynamically generated skills
- Self-Improvement from Ratings ✅ DONE — rating-based behavior adjustment
- Onchain Reputation System ✅ DONE — verifiable reputation from work
- Mobile App Framework ✅ DONE — push notifications and session summaries

### Tier 3 (Long-term - ALL COMPLETE)
- Zod-typed Tool Outputs ✅ DONE — JSON schema generator with OpenAPI compatibility
- Voice I/O (STT/TTS) ✅ DONE — OpenAI TTS-1 + ElevenLabs TTS, Whisper/Deepgram/Groq STT
- Local Embedding Caching ✅ DONE — wired in orient() phase, load/save/get/put
- Embedding Provider System ✅ DONE — OpenAI + local hash, cache integration, cosine similarity
- Scheduled Event Triggers ✅ DONE — cron evaluation, And/Or/Not conditions, HMAC webhooks, chains

### New Features (2026-05-11)
- Rate Limiter ✅ DONE — token bucket, per-tool + global, dynamic registration
- Health Monitor ✅ DONE — subsystem checks, alerting, Healthy/Degraded/Unhealthy
- Session Search ✅ DONE — full-text search across session outcomes and summaries (storage/search.rs)
- Tool Pipeline ✅ DONE — declarative multi-step chains, variable resolution, failure policies
- Cost Tracker ✅ DONE — per-session/tool/model, 15 known model prices, token summaries

### New Features (2026-05-12)
- Circuit Breaker ✅ DONE — closed/open/half-open states, failure threshold, timeout, registry
- Response Cache ✅ DONE — SHA256 content-addressable, TTL, hit/miss stats, token savings
- Graceful Shutdown ✅ DONE — ShutdownFlag, cleanup hooks, grace period
- Wave Execution ✅ DONE — dependency-aware parallel wave scheduling (src/wave/)
- WASM Sandbox ✅ DONE — wasmtime isolation with capability-based permissions (src/wasm/)

---

## Hermes ↔ Praxis Gap Analysis (from GAP_ANALYSIS_HERMES.md)

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

## Test Status

**395 tests passing, 1 ignored, 0 failures.**

---

## Remaining for Competition ($20k prize)

All core features complete. All Shelldex Tiers 1-3 complete.
5 additional features added 2026-05-11 (rate limiter, health monitor, session search, tool pipeline, cost tracker).
3 additional features added 2026-05-12 (circuit breaker, response cache, graceful shutdown).
Wave execution and WASM sandbox wired 2026-05-12.

External blockers only:
- Vercel Sandbox SDK — no SDK available
- Discord Voice/opus crate — not available

---

## File Inventory

```
Root docs:
 STATUS.md ← this file
 PRAXIS_DESIGN.md — architecture doc (1400 lines)
 README.md — project overview
 CLAUDE.md — dev guide for AI assistants
 CONTRIBUTING.md — contribution guidelines

src/ module directories (74 dirs, all with mod.rs):
 a2a/ anatomy/ anomaly/ archive/ argus/ attachments/
 backend/ bench/ boundaries/ brief/ bus/ canary/
 canvas/ cli/ config/ context/ crypto/ curator/
 daemon/ dashboard/ delegation/ embedding/ events/ evolution/ examples/
 federation/ forensics/ hands/ heartbeat/ hooks/ i18n/ identity/
 kanban/ learning/ lite/ loop/ main/ marketplace/ mcp/
 meet/ memory/ merkle/ messaging/ oauth/ observability/
 paths/ plugins/ plugin_signing/ postmortem/ profiles/ providers/ quality/
 report/ rules/ sandbox/ score/ session/ skills/ speculative/
 spotify/ state/ storage/ time/ tools/ tui/
 usage/ vault/ voice/ wakeup/ wave/ webhook/ webhooks/
 watchdog/ (standalone binary — src/watchdog/main.rs)
 wasm/ (feature-gated — Cargo.toml [features] wasm = ["dep:wasmtime"])

src/ standalone module files (.rs, not directories):
 backup.rs  browser.rs  browser_pwa.rs  channels.rs  checkpoints.rs
 circuit_breaker.rs  cost.rs  crypto.rs  docker_isolation.rs  embedding_cache.rs
 gitclaw.rs  graceful_shutdown.rs  health.rs  hotreload.rs  injection.rs
 leaks.rs  mobile.rs  onchain_reputation.rs  openmolt.rs
 personality.rs  pipeline.rs  proactive.rs  process_manager.rs
 rate_limit.rs  rating_improve.rs  report.rs  response_cache.rs  routines.rs
 rrf.rs  runtime_skill.rs  self_update.rs  skill_pack.rs
 tool_schema.rs  tracing.rs  trigger.rs  zeptoclaw.rs  zh_channels.rs
```

---
