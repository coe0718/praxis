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
- Scheduled Event Triggers ✅ EXISTS (src/tools/cron_ext.rs)
- 32 Built-in Tools (ZeptoClaw) ✅ DONE — 32 tools across 6 categories
- Chinese Platform Channels ✅ DONE (QQ, Feishu, DingTalk, WeChat, WeCom)
- Chinese LLM Providers ✅ N/A — integrated via provider system
- Runtime Skill Creation ✅ DONE — dynamically generated skills
- Self-Improvement from Ratings ✅ DONE — rating-based behavior adjustment
- Onchain Reputation System ✅ DONE — verifiable reputation from work
- Mobile App Framework ❌ MISSING — listed as exists but src/mobile.rs not found

### Tier 3 (Long-term - 11+ features)
- Zod-typed Tool Outputs ✅ DONE — JSON schema generator with OpenAPI compatibility
- Local STT/TTS (whisper.cpp)
- Local Embedding Caching
- Embedding Provider System
- Scheduled Event Triggers (enhanced)
- And 7 more...

---

## Hermes ↔ Praxis Gap Analysis (from GAP_ANALYSIS_HERMES.md)

All 10 items from the OpenClaw gap analysis are now closed:

|| # | Feature | Status | Implementation |
||---|---------|--------|----------------|| 1 | Kanban | ✅ | `src/kanban/` — full board with dispatcher + workers |
|| 2 | Sessions spawn | ✅ | `src/session/spawn.rs` — programmatic creation |
|| 3 | Curator | ✅ | `src/curator/mod.rs` — run_cycle wired |
|| 4 | 429 fallback | ✅ | `src/backend/retry.rs` — exponential backoff |
|| 5 | Cron extensions | ✅ | `src/tools/cron_ext.rs` — no_agent, wake_gate |
|| 6 | Live Canvas | ✅ | `src/canvas/mod.rs` — streaming HTML surface |
|| 7 | LanceDB memory | ✅ | `src/memory/lance.rs` — vector semantic recall |
|| 8 | Auto-reply | ✅ | `src/messaging/auto_reply.rs` — rate-limited responses |
|| 9 | i18n | ✅ | `src/i18n/mod.rs` — 9 languages |
|| 10 | Plugin registry | ✅ | `src/plugins/marketplace.rs` — remote discovery |

---

## External Integrations

|| Integration | Status | Notes |
||-------------|--------|-------|
|| Telegram | ✅ Working | Polling bot, morning brief, daily gate |
|| Discord | ✅ In+Out | `discord.rs` outbound + `inbound.rs` REST polling |
|| Slack | ✅ In+Out | `slack.rs` outbound + `inbound.rs` REST polling |
|| Spotify | ✅ Working | PKCE OAuth2, 8 actions |
|| Google Meet | ✅ Working | OAuth2 device flow, 4 actions |
|| Langfuse | ✅ Wired | Real HTTP client |
|| Prometheus | ✅ Wired | `/metrics` endpoint |
|| Vercel | 🔴 Blocked | No vercel.com project/SDK |

---

## Remaining for Competition ($20k prize)

All core features complete. Shelldex features in pipeline for Q2-Q3 2026.

---

## File Inventory

```
Root docs:
 STATUS.md ← this file (supersedes all prior gap/feature docs)
 SHELLDEX_FEATURE_HARVEST.md — reference for 28+ future features
 PRAXIS_GAPS.md → superseded, kept for history
 PRAXIS_DESIGN.md → architecture doc (keep)
 README.md → overview (keep)
 CLAUDE.md → dev guide (keep)

src/ module directories (all with mod.rs):
 a2a/ anatomy/ anomaly/ archive/ argus/ attachments/
 backend/ bench/ boundaries/ brief/ bus/ canary/
 carapace/ canvas/ cli/ config/ context/ crypto/ curator/
 daemon/ dashboard/ delegation/ events/ evolution/ examples/
 federation/ forensics/ hands/ heartbeat/ hooks/ i18n/ identity/
 kanban/ learning/ lib/ lite/ loop/ main/ mcp/
 meet/ memory/ merkle/ messaging/ oauth/ observability/
 onchain_reputation/ openmolt/ paths/ personality/ plugins/ profiles/ providers/ quality/
 rating_improve/ report/ runtime_skill/ rules/ sandbox/ score/ session/ skills/ speculative/
 spotify/ state/ storage/ time/ tools/ tui/
 usage/ vault/ wakeup/ wave/ webhook/ webhooks/
 zeptoclaw/ zh_channels/
```

---

_Last updated: 2026-05-08 — Drey_