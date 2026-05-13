# Hermes Agent → Praxis Port Analysis

**Author:** Scout  
**Date:** 2026-05-12  
**Context:** Analysis of porting Hermes Agent (Python, 3,374 files, 103MB) to Rust, then adding all Praxis features to it.

---

## Executive Summary

**The premise is backwards.** Praxis is already a Rust agent with a superior architecture. The correct move is not "port Hermes to Rust and add Praxis features" — it's "absorb remaining Hermes features into Praxis."

| Metric | Hermes Agent | Praxis |
|--------|-------------|--------|
| Language | Python (1,628 .py files) | Rust (327 .rs source files) |
| Total size | 103MB, 3,374 files | ~19MB, ~20K files (mostly target/) |
| Tests | ~17,000 (Python/pytest) | 395 passing (Rust/integration) |
| Agent loop | Synchronous `run_conversation()` | 5-phase: Orient→Decide→Act→Reflect→Sleep |
| Core features | Plugin system, gateway, tools, skills, cron, sub-agents | Everything Hermes has + 40+ additional features |
| CLI | Python (prompt_toolkit + Rich) | Rust (clap) |
| Messaging gateway | 18 platform adapters (Python/asyncio) | 3 platforms (Telegram, Discord, Slack) |
| Learning loop | Skill synthesis from experience | Full self-evolution + quality gates + reviewer + score |
| Distribution | `pip install` or `curl` install.sh | Single Rust binary |

---

## The Real Situation: Praxis vs Hermes Feature Overlap

### Features Hermes Has That Praxis Does NOT Yet Have

These are the features Praxis should absorb from Hermes. Organized by priority.

#### Critical (build-or-port)

| Feature | Hermes Module(s) | Praxis Impact | Port Effort |
|---------|-----------------|---------------|-------------|
| **17 more messaging platforms** | `gateway/platforms/` (Signal, WhatsApp, Matrix, Mattermost, email, SMS, BlueBubbles, WeChat, WeCom, Feishu, DingTalk, QQ, Yuanbao, HomeAssistant, API server, webhook) | Massive reach expansion | **High** — each is its own adapter |
| **18+ model providers** | `plugins/model-providers/` (Anthropic, Bedrock, Gemini, DeepSeek, xAI, MiniMax, Nous, NVIDIA, HuggingFace, Z.AI, Xiaomi, StepFun, Alibaba, Kimi, Ollama Cloud, Azure, GMI, custom) | Praxis has 3 (OpenAI, Claude, Ollama) | **Medium** — mostly OpenAI-compatible |
| **40+ built-in skills** | `skills/` (p5js, manim, excalidraw, notion, spotify, google-workspace, obsidian, linear, git, etc.) | Rich pre-built capability library | **Low** — TOML manifests + shell scripts |
| **Skills Hub** | `tools/skills_hub.py`, `hermes_cli/skills_hub.py`, `tools/skills_sync.py` | Discovery & install from agentskills.io | **Medium** |
| **Sub-agent delegation** | `tools/delegate_task.py`, `model_tools.py` | Praxis has this via `src/delegation/` but Hermes has richer patterns | **Low** |
| **Cron scheduler** | `cron/` directory, `tools/cronjob_tools.py` | Praxis has `tokio-cron-scheduler` — Hermes has richer cron-tool integration | **Low** |
| **Browser automation** | `tools/browser_camofox.py`, `tools/browser_cdp_tool.py`, `tools/browser_tool.py`, `tools/browser_providers/` | `src/browser.rs` exists but is basic | **Medium** |
| **TUI (React Ink)** | `ui-tui/` (Ink + React + TypeScript) | Praxis has `ratatui` TUI — different approach entirely | **N/A** — use Praxis's ratatui |
| **Image generation** | `plugins/image_gen/` (OpenAI, xAI), `tools/image_generation_tool.py` | Praxis can add OpenAI endpoint | **Low** |
| **Vision tools** | `tools/vision_tools.py` | `src/tools/vision.rs` exists | **Low** |
| **Voice/transcription** | `tools/transcription_tools.py`, `tools/tts_tool.py`, `tools/voice_mode.py` | `src/voice/` exists, `src/tools/voice.rs` exists | **Low** |
| **TTS** | `tools/neutts_synth.py`, `tools/tts_tool.py` | Praxis has voice I/O | **Low** |

#### High Value — Unique Hermes IP

| Feature | Hermes Module(s) | Notes |
|---------|-----------------|-------|
| **LSP support** | `agent/lsp/` — Language Server Protocol client for code editing | Unique to Hermes. Praxis has no code editor integration |
| **ACP adapter** | `acp_adapter/`, `acp_registry/` — ACP server for VS Code/Zed/JetBrains | Unique UX integration |
| **7 terminal environments** | `tools/environments/` — local, Docker, SSH, Modal, Daytona, Singularity, Vercel Sandbox | Praxis only has `docker_isolation.rs` and `src/delegation/` |
| **Plugin marketplace** | `tools/skills_hub.py`, `hermes_cli/skills_hub.py` | Published skills ecosystem |
| **Achievement system** | `plugins/hermes-achievements/` | Gamification layer |
| **RL training environments** | `environments/` (Atropos, SWE-bench, terminal_test_env) | Research-focused |
| **Trajectory capture** | `agent/trajectory.py` | Training data generation |
| **Batch runner** | `batch_runner.py` | Parallel batch processing for research |
| **`hermes doctor`** | `hermes_cli/doctor.py` | Diagnostic CLI tool |
| **`hermes setup` wizard** | `hermes_cli/setup.py` | Interactive setup with model interview |
| **`hermes update`** | `hermes_cli/pluggable_commands.py` | Self-update mechanism |
| **`hermes claw migrate`** | `hermes_cli/claw.py` | Migration from OpenClaw |
| **Config wizard** | `hermes_cli/config.py`, `hermes_cli/setup.py` | Interactive config management |
| **AI model catalog** | `hermes_cli/model_catalog.py`, `hermes_cli/models.py`, `agent/models_dev.py` | Provider/model discovery |
| **Web dashboard chat pane** | `gateway/web_server.py`, `gateway/assets/` | Browser-based chat (uses PTY — needs WSL2 on Windows) |

### Features Hermes Has That Praxis Also Has (equivalents)

These are direct equivalents — Praxis already has them.

| Hermes Feature | Praxis Equivalent | Notes |
|---------------|-------------------|-------|
| Plugin system | `src/plugins/`, `src/plugin_signing/` | Praxis supports `.so` dynamic libs + TOML manifests |
| Memory management | `src/memory/`, `src/storage/sqlite/memory*.rs` | Praxis has hot/cold/operational memory |
| Tool registry | `src/tools/registry.rs` | TOML manifest-based tools |
| SQLite storage | `src/storage/sqlite/` | rusqlite with FTS5 |
| Kanban board | `src/kanban/` | Dispatcher + workers |
| MCP support | `src/mcp/` | Server + client |
| Skill system | `src/skills/`, `src/runtime_skill.rs` | SKILL.md frontmatter + synthesis |
| i18n | `src/i18n/` | 9 languages |
| Identity files | `src/identity/` | SOUL.md / IDENTITY.md |
| Slack integration | `src/messaging/slack.rs` | |
| Discord integration | `src/messaging/discord.rs` | |
| Telegram integration | `src/messaging/telegram.rs` | |
| Web search | `src/tools/` — `web-fetch` tool | |
| File operations | `src/tools/` — `file-read`, `shell-exec` | |
| Image generation | `src/tools/image.rs` | |
| Observability (Langfuse) | `src/observability/langfuse.rs` | |
| Spotify integration | `src/spotify/` | |
| Google Meet | `src/meet/` | |

### Features Praxis Has That Hermes Does NOT

These are Praxis-exclusive differentiators — the features that make Praxis the superior platform. If you "ported Hermes to Rust," you'd still need to build all of these.

#### Core Agent Loop & Intelligence
- **5-phase self-evolving loop** (Orient → Decide → Act → Reflect → Sleep) — Hermes has a single `run_conversation()` loop
- **Self-evolution system** — `src/evolution/` generates proposals from session outcomes, auto-adjusts identity
- **Quality gates** — `src/quality/gates.rs` — deterministic output validation before operator delivery
- **Reviewer system** — `src/quality/reviewer.rs` — independent reviewer, no self-review rule
- **Success criteria JSON** — `goals/criteria/{goal-id}.json` — deterministic verification per goal
- **Evaluate loop** — `src/quality/evaluate.rs` — generator/evaluator closed-loop
- **Brier score calibration** — prediction accuracy tracking
- **Irreplaceability scoring** — `src/score/` — 4-dimension composite per session
- **Argus drift detection** — `src/argus/` — cross-session failure clustering, pattern mining

#### Security & Isolation
- **WASM sandbox** — `src/wasm/` — wasmtime with capability-based permissions, fuel metering
- **Circuit breaker** — `src/circuit_breaker.rs` — closed/open/half-open with auto-recovery
- **Rate limiter** — `src/rate_limit.rs` — token bucket, per-tool + global
- **Dynamic trust budget** — per-capability-class trust expansion/contraction
- **Channel-scoped sandboxes** — Docker isolation per channel
- **Loop guard** — SHA256 consecutive-tool-hash detection + cross-tool loop detection
- **Credential vault proxy** — `src/vault/` — secrets via env-var references, not literal values
- **Zeroize secrets** — `src/crypto/zeroize.rs` — secure memory clearing
- **File-mutation circuit breaker** — write-path fanout caps
- **Ed25519 signed tool manifests** — `src/tools/manifest.rs`
- **Sender pairing codes** — per-platform pairing/approval
- **Boundary memory** — hard "never do this" rules enforced before Act begins

#### Agent Loop Enhancements
- **Speculative execution** — `src/speculative/` — multi-branch rehearsal with trust constraints
- **Wave execution** — `src/wave/` — dependency-aware parallel wave scheduling
- **Context handoff notes** — structured checkpoints for long goals
- **Progressive skill disclosure** — compact catalog → full load on demand
- **Anatomy index + repeated-read detection** — `src/anatomy/`
- **Loop steering** — `src/loop/steer.rs` — redirect the agent mid-session
- **Context adaptive allocation** — auto-tunes source priorities from outcomes

#### Memory & Learning
- **2-tier memory** (hot/cold) with TTL, decay, reinforcement, consolidation
- **Memory conflict detection** — `src/memory/conflicts.rs` — contradiction workbench
- **Preference graph** — structured operator model with confidence + provenance
- **Operational memory** — do-not-repeat register + known-bug log
- **Learning runtime** — `src/learning/` — scheduled opportunity mining from approved sources
- **Embedding system** — `src/embedding/` — OpenAI + local hash, cosine similarity
- **Response cache** — `src/response_cache.rs` — SHA256 content-addressable
- **Synthetic example generation** — `src/examples/`

#### Infrastructure
- **Two-process watchdog** — `src/watchdog/` — monitors main process, handles binary swaps mid-sleep
- **Self-update with canary** — `src/self_update.rs` — rolls back if canary fails
- **Auto-git state sync** — `src/cli/git.rs` — auto-commits every Reflect
- **Prometheus metrics** — `/metrics` endpoint on dashboard
- **SSE event streaming** — `/events` real-time stream
- **Cost tracker** — `src/cost.rs` — per-session/tool/model with 15 known model prices
- **Health monitor** — `src/health.rs` — subsystem checks, Healthy/Degraded/Unhealthy
- **Token ledger** — per-phase, per-provider token tracking
- **Forensics + replay** — `src/forensics/` — time-travel session reconstruction
- **Postmortem generator** — structured POSTMORTEMS.md entries
- **Hierarchical goals** — parent/child goal relationships
- **Model profiles** — quality/budget/deterministic execution profiles
- **Canary runs** — `src/canary/` — provider health verification
- **Quiet hours + lane-based scheduling** — concurrency isolation per work type
- **Agent-as-worker marketplace** — `src/marketplace/` — CashClaw-style work items + reputation
- **Onchain reputation** — `src/onchain_reputation.rs`
- **Git-native lifecycle** — `src/gitclaw.rs` — version-controlled identity/rules/memory
- **Chinese platform channels** — `src/zh_channels.rs` — QQ, Feishu, DingTalk, WeChat, WeCom
- **Mobile app framework** — `src/mobile.rs` — push notifications, session summaries
- **Vault + OAuth system** — `src/oauth/` — GitHub, Google, Gmail, device flow, Copilot
- **Federation** — `src/federation/` — multi-instance coordination
- **Portable state export/import** — `src/archive/` — manifest-versioned bundles
- **Weekly alignment review** — structured operator alignment checkpoint
- **Annual synthesis** — YEAR_IN_REVIEW.md / BIOGRAPHY.md

---

## Port Analysis: What Would It Mean?

### Option A: Port Hermes to Rust (The Original Question)

**Verdict: Do not do this.** It's a waste of time. Here's why:

1. **You'd be rebuilding Praxis from scratch.** Praxis already has the Rust runtime, SQLite schema, provider abstraction, orchestration loop, tool system, and 70+ module directories. Porting Hermes's Python features to Rust would mean reimplementing what Praxis already has.

2. **Hermes is 15,936 lines in `run_agent.py` alone** — a single-file Python agent loop. Praxis's equivalent is spread across `src/loop/` (6 files), `src/context/` (12 files), `src/tools/` (20 files), and is far more modular and testable.

3. **Hermes's strength is its plugin ecosystem and platform breadth** — these are configuration-level features, not architectural ones. You don't need to "port" them; you need to absorb them.

### Option B: Absorb Hermes Features Into Praxis (Recommended)

**The correct approach.** Praxis is the Rust platform. Hermes is a feature source.

#### Phase 1: Messaging Platform Explosion (Highest ROI)

Hermes has 18 messaging platform adapters. Praxis has 3. This is the biggest capability gap.

**Strategy:** Don't port Hermes's Python adapters. Use their protocol knowledge to write Rust equivalents:

- **Signal**: Signal CLI + REST. Use `reqwest` + Signal JSON-RPC.
- **WhatsApp**: WhatsApp Cloud API (Meta). Straightforward REST.
- **Matrix**: Matrix HTTP API. `reqwest` + sync endpoint.
- **Mattermost**: Mattermost API. Straightforward REST.
- **SMS**: Twilio/SNS. Simple POST.
- **Email**: SMTP/IMAP (lettre crate for SMTP, async-imap for IMAP).
- **BlueBubbles**: Private API reverse-engineered — skip unless needed.
- **HomeAssistant**: REST API + WebSocket — add as a tool, not a platform.
- **QQ/Feishu/DingTalk/WeChat/WeCom**: Already in `src/zh_channels.rs` ✅

**Effort:** ~2-3 days per adapter for the first few, ~1 day each after patterns stabilize.

#### Phase 2: Model Provider Expansion

Hermes has 17+ model provider plugins. Praxis has 3 (OpenAI, Claude, Ollama).

**Strategy:** Praxis's `ProviderProtocol` + `reqwest` adapter pattern already handles most of this. For any OpenAI-compatible provider, it's a config entry, not code.

- **Already OpenAI-compatible** (zero code): DeepSeek, xAI, MiniMax, Nous, NVIDIA NIM, HuggingFace TGI, Z.AI, Xiaomi, StepFun, Alibaba, Kimi, OpenRouter
- **Needs adapter work**: Gemini (native adapter), Bedrock (AWS SDK), Anthropic (already done), Azure (different auth)

**Effort:** ~1 day for per-provider config validation. Most are zero-code.

#### Phase 3: Skills Hub & Tool Catalog

Hermes ships 40+ SKILL.md files. Praxis's skill system (`src/skills/`) uses the same format.

**Strategy:** Copy the SKILL.md files into Praxis's `skills/` directory. They're portable by design. The TOML frontmatter + markdown body format is shared.

**Effort:** ~1 hour to copy and verify.

#### Phase 4: Terminal Environment Expansion

Hermes supports 7 terminal backends. Praxis only has `docker_isolation.rs`.

| Hermes Backend | Praxis Equivalent | Port Effort |
|---------------|-------------------|-------------|
| local | `shell-exec` tool ✅ | Already works |
| Docker | `docker_isolation.rs` ✅ | Already works |
| SSH | None | **1-2 days** — `ssh2` crate or `tokio::process::Command` over SSH |
| Modal | None | **2-3 days** — REST API calls |
| Daytona | None | **1-2 days** — REST API calls |
| Singularity | None | **1 day** — can build on docker_isolation |
| Vercel Sandbox | None | Blocked — no SDK available |

**Effort:** ~5-10 days total for all backends.

#### Phase 5: Developer UX & Quality-of-Life Features

Hermes has: `hermes doctor`, `hermes setup`, `hermes update`, `hermes claw migrate`, model catalog.

Praxis already has: `praxis init`, `praxis doctor` (subset), `praxis git`, `praxis model`, `praxis models`.

**Gaps:**
- **`praxis update`** — self-update. `src/self_update.rs` exists ✅
- **`praxis doctor`** — diagnostic checks. Expand the existing CLI.
- **Interactive setup wizard** — `src/cli/wizard.rs` exists ✅
- **Model catalog** — provider discovery endpoint
- **Migration from OpenClaw/Hermes** — import state bundles

**Effort:** ~3-5 days.

---

## If the Question Is Really "What Would It Take to Port HERMES to Rust?"

**Size estimate: ~100,000-150,000 lines of Rust** to achieve feature parity with what Hermes does today. At a professional pace (~500 lines/day), that's 200-300 days. At your pace (one person + AI), maybe 60-90 days with heavy AI assistance.

But you'd end up with something that's still behind Praxis.

---

## Recommended Action Plan

1. **Don't port Hermes.** Praxis is already better in every architectural dimension.
2. **Absorb Hermes messaging platforms** — add Signal, WhatsApp, Matrix, Mattermost, Email, and SMS to `src/messaging/`. These are the highest-value gap.
3. **Copy Hermes skills** — the 40+ SKILL.md files are fully portable. Install them in `skills/`.
4. **Add SSH backends** — the `ssh2` crate + `tokio::process::Command` for remote exec.
5. **Use Hermes's behavioral patterns** — progressive skill disclosure (already in Praxis), LSP/code editing patterns, and the `hermes doctor` setup are useful UX lessons.
6. **Build a `praxis doctor` command** — system health diagnostic like Hermes's `hermes doctor`.
7. **Build a `praxis update` flow** — `src/self_update.rs` + `praxis update` CLI integration.

---

---

## The Always-On Gap

This is the most important UX difference between the two, and the one that matters most for "I can message you any thing I want and you always answer."

### Current State: Praxis Daemon

`praxis daemon` *is* always-on in architecture, but with a critical gap:

| Aspect | Praxis | Hermes | Impact |
|--------|--------|--------|--------|
| **Reception model** | Polling (default 30s interval) | Webhooks + long-polling + WebSockets | **30-second worst-case delay** before Praxis sees a message |
| **Platform count** | 3 (Telegram, Discord, Slack) | **18+** (all of the above + Signal, WhatsApp, Matrix, email, SMS, QQ, Feishu, DingTalk, WeChat, WeCom, BlueBubbles, Yuanbao, HomeAssistant, API server, webhook) | Praxis only meets you on platforms you might already use. Hermes meets you on whatever you're already on. |
| **Push vs Poll** | Poll — every 30s, asks "any new messages?" | Webhook-supported (Telegram) + WebSocket (Discord Gateway) | Hermes gets messages as they happen. Praxis discovers them up to 30s later. |
| **Session trigger** | Bus watcher → session starts on next poll tick | Immediate handler invocation | Sub-second response vs up-to-30-second delay |
| **Cross-platform continuity** | Per-platform state files | Unified gateway session model | Hermes threads conversations across platforms seamlessly |
| **Multi-instance** | Single process, single data dir | Gateway manages per-chat AIAgent instances | Hermes handles multiple concurrent conversations independently |

### How Hermes Achieves "Always On"

Hermes's `gateway` is a separate daemon process that:

1. **Holds persistent connections** — Discord WebSocket Gateway, Telegram webhook receiver, Signal CLI daemon. These are *push* connections that deliver messages as they happen, not polling loops.

2. **Maintains a per-session AIAgent cache** — each chat gets its own agent instance kept warm in an LRU cache. Your message hits the gateway, gets routed to the right AIAgent, and you get a response — all without spinning up a new agent from scratch.

3. **Handles 18+ platforms from one process** — `python gateway/run.py` starts one process that connects to every configured platform simultaneously. Messages from any platform enter the same routing pipeline.

4. **Has a `hermes gateway setup` wizard** — walks you through API key entry, webhook configuration, and platform registration in one session.

### The Key Insight

**Praxis already has the bones of "always-on"** — it's not missing the daemon or the reactive trigger model. What it's missing is **push infrastructure** and **platform breadth**:

- **Praxis polls Telegram** (30s interval). Hermes uses **Telegram webhooks** (instant push).
- **Praxis polls Discord REST API**. Hermes uses **Discord Gateway WebSocket** (real-time push).
- **Praxis polls Slack REST API**. Slack also supports **Events API webhooks** (push).
- **Praxis has 3 platforms**. Hermes has 18.

### What to Build for "Always-On" Praxis

#### Phase 1: Switch from Poll to Push (High ROI, ~3 days)

| Platform | Current | Target | Method |
|----------|---------|--------|--------|
| Telegram | Poll every 30s | **Webhook** | `setWebhook` API → axum `/webhook/telegram` endpoint → BusEvent |
| Discord | Poll every 30s | **Gateway WebSocket** | `tokio-tungstenite` → Gateway intents → message events |
| Slack | Poll every 30s | **Events API** | axum `/webhook/slack` → URL verification + event subscription |

All three push endpoints can live on the same axum HTTP server that already serves the dashboard. No new binary, no new deployment.

#### Phase 2: Add Push-Only Platforms (~5-7 days)

| Platform | Method | Notes |
|----------|--------|-------|
| WhatsApp Cloud API | Webhook | Meta sends POST to your webhook URL. Reply via Graph API. |
| Matrix | WebSocket + Sync API | `/_matrix/client/v3/sync` with long-polling. Crate: `matrix-sdk`. |
| Signal | Signal Messenger REST API | Polling or webhook with Signal's JSON-RPC. |
| Email (IMAP IDLE) | Push | `async-imap` crate with IDLE extension. True push for new mail. |

#### Phase 3: Reduce Poll Interval for Remaining Platforms

For platforms that don't support push (SMS, BlueBubbles, legacy adapters), the poll interval should be configurable down to 1-2 seconds for near-real-time response. The daemon already supports `--poll-interval`.

### "Always On" Architecture for Praxis

```
                    ┌──────────────────────┐
                    │  axum HTTP Server     │
                    │  (dashboard + hooks)  │
                    └──────┬───────────────┘
                           │
  ┌────────────┐     ┌─────▼──────────────┐     ┌─────────────┐
  │ Telegram   │────▶│ Webhook handlers   │────▶│             │
  │ Webhook    │     │                    │     │             │
  └────────────┘     │ /webhook/telegram  │     │   Message   │
                     │ /webhook/slack     │     │    Bus      │
  ┌────────────┐     │ /webhook/whatsapp  │     │ (bus.jsonl) │
  │ Slack      │────▶│ /webhook/...       │     │             │
  │ Events API │     └────────────────────┘     └──────┬──────┘
  └────────────┘                                      │
                                                      │ reads
  ┌────────────┐     ┌────────────────────┐     ┌──────▼──────┐
  │ Discord    │────▶│ tokio-tungstenite  │────▶│ Daemon Loop │
  │ Gateway WS │     │ WebSocket client   │     │ (30s poll   │
  └────────────┘     └────────────────────┘     │  + reactive)│
                                                └──────┬──────┘
                                                       │
                                                ┌──────▼──────┐
                                                │  run_once() │
                                                │  (session)  │
                                                └─────────────┘
```

The key change: push handlers write directly to the Bus. The daemon's `BusWatcher` already detects new bus events and triggers a reactive session. No daemon code changes needed — just wire the webhook endpoints to pulse the bus.

---

## Bottom Line

Praxis already has **~80% of Hermes's feature surface** plus **40+ features Hermes doesn't have at all**. The "port Hermes to Rust" question is based on an outdated premise. Praxis isn't a port target — it's the destination.

**Hermes is a feature source, not a migration source.** Pull in what's missing (messaging breadth → push/webhook architecture, terminal backends, skills hub), skip the rest. Praxis already won.

The "always-on" gap is the single highest-impact thing to fix — because it's not about code volume, it's about making Praxis feel as responsive as Hermes. Three webhook handlers and a WebSocket client would close 90% of the latency gap.
