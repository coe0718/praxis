# Praxis vs Hermes Agent — Gap Analysis

*Generated 2026-04-25 by Drey. Updated 2026-04-25 to reflect wave 1-3 closures.*

---

## Overview

Praxis is a mature Rust agent framework with ~170 source files covering a 4-phase loop, multi-provider backend, memory, skills, dashboard, MCP, and more. This document enumerates what Hermes Agent can do that Praxis cannot yet do — **33 gaps identified** after cross-referencing the GitHub README, official docs, and tools reference. Items are ranked: 🔴 Critical → 🟠 High Impact → 🟡 Nice-to-Have.

---

## What Praxis ALREADY Has (for context)

| Category | Praxis Capability |
|---|---|
| Agent loop | Orient → Decide → Act → Reflect → Sleep |
| Providers | Claude, OpenAI, Ollama, Router + custom OpenAI-compat |
| Streaming | Async SSE streaming backend (OpenAI-compat) |
| Prompt caching | Anthropic prompt caching enabled |
| Memory | Hot/cold tiers, consolidation, contradiction detection, milestone protection |
| Skills | Local SKILL.md files with TOML frontmatter, catalog loading, synthesis |
| Execution profiles | quality, budget, offline, deterministic, lite |
| Cron scheduling | tokio-cron-scheduler via watchdog binary |
| Messaging | Telegram (polling+sending), Discord (poll+webhook), Slack (poll+webhook) |
| Dashboard | SPA with 24 API routes, SSE events, Prometheus |
| MCP | Server (tools/list, tools/call) + Client (HTTP) |
| Evolution | Agent self-proposals (Config + Identity) |
| Delegation | File-based agent-to-agent via WakeIntent |
| Quality | Reviewer sub-agent, evals, gates, scoring |
| Tools | file-read, git-query, shell-exec, web-fetch + TOML manifest system |
| OAuth | GitHub, Gmail, Copilot via device flow |
| TUI | ratatui terminal dashboard |
| Watchdog | Binary update + rollback |
| Vault | Credential store with env-var injection |
| Sandbox | Per-channel filesystem isolation |
| Speculative execution | Branch rehearsal before commit |
| Forensics | Session snapshots, Merkle audit trail |
| Pairing | Sender pairing flow for Telegram |
| Typing indicators | Trait for platform adapters |
| A2A | Client types and basic HTTP client |

---

## 🔴 Critical Gaps

### 1. Voice / STT (Speech-to-Text) / TTS (Text-to-Speech)
**Status:** STUB — zero implementation. Roadmap and NEEDS_FINISHED.md both list this.  
**Hermes equivalent:** Local faster-whisper (free), Groq Whisper (free tier), OpenAI Whisper, Mistral Voxtral for STT. Edge TTS (free default), ElevenLabs, OpenAI, MiniMax, NeuTTS for TTS. `/voice on` command for voice-to-voice mode.  
**Why it matters:** Voice is the most natural input modality for mobile/ambient use. Without it, Praxis can't receive voice memos on Telegram/WhatsApp or speak responses back.  
**Effort:** Medium. Rust has `whisper-rs` crate. TTS could use Edge TTS HTTP API or `neutts`.

### 2. Vision / Multi-Modal Support
**Status:** Not implemented. No image input path exists.  
**Hermes equivalent:** `vision_analyze` tool — analyzes images via AI vision. Supports URLs and local files.  
**Why it matters:** Claude and GPT-4o support image input. Operators send screenshots, photos, documents. Without vision, Praxis is text-only.  
**Effort:** Medium. Add `image_url` / `base64_image` fields to provider requests. Requires content block arrays instead of plain text messages.

### 3. Config Hot-Reload
**Status:** ✅ Implemented. `src/config/watcher.rs` uses `notify` crate to watch `praxis.toml`. Daemon detects changes within 2s and re-validates. Session runner already re-reads config per-cycle.  
**Hermes equivalent:** Config changes take effect on gateway `/restart` or CLI relaunch.  
**Why it matters:** An always-on daemon shouldn't require a restart to change backend, budget, or security settings.  ✅ *Done. Wave 4.*

---

## 🟠 High-Impact Gaps

### 4. Code Execution (Sandboxed)
**Status:** Not implemented. `shell-exec` tool exists but runs raw `/bin/bash -c` — no Python sandbox, no isolated runtime.  
**Hermes equivalent:** `execute_code` tool — runs Python scripts with access to Hermes tools. `code_execution` toolset for sandboxed execution.  
**Why it matters:** Agents need to write and run code to be useful for data analysis, automation, and self-improvement. Shell is not enough.  
**Effort:** High. Would need a Python runtime integration or WASM sandbox.

### 5. Browser Automation
**Status:** Not implemented.  
**Hermes equivalent:** `browser` toolset — Browserbase, Camofox, or local Chromium. Full browser_navigate, browser_click, browser_snapshot, browser_console, browser_vision.  
**Why it matters:** Web interaction is a core agent capability — research, form filling, scraping, testing.  
**Effort:** High. Would need headless Chrome + CDP integration in Rust, or delegate to an external service.

### 6. Persistent User Memory Across Sessions
**Status:** ✅ Implemented. `src/memory/user.rs`, `user_memory.json`, `memory` tool with upsert/search/forget/list + tags. Survives across sessions.  
**Hermes equivalent:** `memory` tool with `user` and `memory` targets. Survives across sessions. Pluggable backends (built-in, Honcho, Mem0). `session_search` for recalling past conversations.  
**Why it matters:** "Remember I prefer X" should persist forever.  ✅ *Done. Wave 2.*

### 7. Credential Pooling / API Key Rotation
**Status:** ✅ Implemented. `src/backend/credential_pool.rs` provides `CredentialPool` with round-robin rotation and per-key 429 cooldown. Keys read from `<PROVIDER>_API_KEY_<N>` env vars. Gated by `features.credential_pooling = true` in `praxis.toml`.  
**Hermes equivalent:** `hermes auth add` for credential pools. Automatic rotation across multiple keys per provider to avoid rate limits.  
**Why it matters:** Rate limits are the #1 cause of agent downtime. Pooling is the standard solution.  ✅ *Done. Wave 5.*

### 8. Cron Job Management as a Tool
**Status:** ✅ Implemented. `src/tools/cron.rs` provides `ScheduledJobs` store with simple schedule expressions (`every 30m`, `in 2h`, `hourly`, `daily`, `weekly`). Agent creates/lists/removes via `cron` tool. Daemon checks due jobs every poll cycle and fires `WakeIntent`. Gated by `features.cron_tool = true`.  
**Hermes equivalent:** `cronjob` tool — create, list, update, pause, resume, remove, run cron jobs from within agent context. Jobs deliver results to messaging platforms.  
**Why it matters:** Self-directed agents should be able to schedule their own recurring tasks.  ✅ *Done. Wave 5.*
**Effort:** Medium. Expose watchdog cron operations as tools in the tool registry.

### 9. Clarify / Ask-User Tool
**Status:** ✅ Implemented. `src/tools/clarify.rs` — pauses agent, writes question to messaging bus, waits for operator response. Supports choice and free-form modes.  
**Hermes equivalent:** `clarify` tool — ask the operator clarifying questions with multiple-choice or free-form options.  
**Why it matters:** Agents hit ambiguity. Without clarify, they guess or fail silently.  ✅ *Done. Wave 1.*

### 10. Todo / In-Session Task Planning
**Status:** ✅ Implemented. `src/tools/todo.rs` — JSON-based task list (create/update/complete/cancel) with persistence.  
**Hermes equivalent:** `todo` tool — create/update/complete task items within a session. Visual task tracking.  
**Why it matters:** Complex tasks need decomposition.  ✅ *Done. Wave 1.*

---

## 🟡 Nice-to-Have Gaps

### 11. Full Profile Isolation
**Status:** Praxis has "execution profiles" (backend routing) but not full profile isolation.  
**Hermes equivalent:** `hermes profile` — create fully isolated profiles with separate configs, sessions, skills, memory. `hermes profile create`, `use`, `delete`, `export`, `import`.  
**Why it matters:** Work vs personal instances, staging vs production, different operators.  
**Effort:** High. Requires multiple data directories and session stores.

### 12. More Messaging Platforms
**Status:** Telegram, Discord, Slack only.  
**Hermes equivalent:** WhatsApp, Signal, Matrix, Email, SMS, Mattermost, Feishu, WeCom, DingTalk, BlueBubbles (iMessage), WeChat, Home Assistant, API Server.  
**Why it matters:** Operator choice. Some operators live on Signal or WhatsApp.  
**Effort:** Variable. Signal/Matrix/WhatsApp would each be significant integrations.

### 13. Plugin System
**Status:** Not implemented. Adding a tool requires editing Rust source and recompiling. Roadmap item #8.  
**Hermes equivalent:** `hermes plugins` — installable plugins for extending functionality. WASM or Python subprocess options discussed in roadmap.  
**Why it matters:** Community contributions, ecosystem growth, operator customization without rebuilds.  
**Effort:** Very High. WASM plugin host or Python embedding.

### 14. ACP / IDE Integration
**Status:** VSCode integration exists (`praxis vscode`) but no ACP (Agent Communication Protocol) support.  
**Hermes equivalent:** `hermes acp` — ACP server for IDE integration with Claude Code, Cursor, Windsurf.  
**Why it matters:** Developers want their agent available in their editor.  
**Effort:** Medium. Implement ACP spec (JSON-RPC over stdio).

### 15. Interactive Session Slash Commands
**Status:** Praxis has CLI subcommands but no interactive session with slash commands (`/help`, `/new`, `/model`, `/compact`, etc.).  
**Hermes equivalent:** 50+ slash commands during interactive sessions — `/new`, `/retry`, `/undo`, `/model`, `/verbose`, `/yolo`, `/rollback`, `/skills`, `/cron`, `/approve`, `/deny`, etc.  
**Why it matters:** Operator experience. Slash commands make the agent feel responsive and controllable.  
**Effort:** Medium. Build an interactive CLI mode (like HermesCLI) with a command registry.

### 16. Checkpoints / Rollback
**Status:** Praxis has forensics snapshots but no operator-facing `/rollback`.  
**Hermes equivalent:** Filesystem checkpoints with `/rollback [N]` to restore previous state.  
**Why it matters:** Safety net for autonomous file modifications.  
**Effort:** Medium. Copy-on-write filesystem checkpointing.

### 17. Worktree / Git Isolation Mode
**Status:** Not implemented.  
**Hermes equivalent:** `-w` / `--worktree` flag for isolated git worktrees — parallel agents don't conflict.  
**Why it matters:** Multi-agent workflows, parallel development.  
**Effort:** Medium. Git worktree integration.

### 18. Setup Wizard
**Status:** ✅ Implemented. Interactive `praxis init` wizard using `dialoguer` crate. Prompts for name, timezone, backend, API key, and security level with defaults and validation. Skipped if CLI flags are provided or config exists.  `src/cli/wizard.rs`.  
**Hermes equivalent:** `hermes setup` — interactive wizard for model, terminal, gateway, tools, agent configuration.  
**Why it matters:** Onboarding. New operators shouldn't need to read TOML docs to get started.  ✅ *Done. Wave 6.*

### 19. Shell Completions
**Status:** ✅ Implemented. `clap_complete` generates bash/zsh/fish completions via `praxis completions <shell>`.  
**Hermes equivalent:** `hermes completion bash|zsh` — generates shell completion scripts.  
**Why it matters:** Operator quality of life.  ✅ *Done. Wave 1.*

### 20. Image Generation
**Status:** ✅ Implemented. `image` tool via `src/tools/image.rs`. Calls OpenAI DALL-E `/v1/images/generations`. Supports `params.prompt`, `params.size` (256x256/512x512/1024x1024), `params.n` (1-4), and `params.provider`. Uses same credential pooling path as chat.  
**Hermes equivalent:** `image_generate` tool — AI image generation from text prompts.  
**Why it matters:** Creative/presentation use cases.  ✅ *Done. Wave 6.*

### 21. Usage Analytics / Insights
**Status:** ✅ Implemented. `praxis insights [--days N]` — token totals, cost estimates, provider breakdown from `token_ledger`.  
**Hermes equivalent:** `hermes insights [--days N]` — cost, token usage, session patterns.  
**Why it matters:** Operators want to know what their agent costs.  ✅ *Done. Wave 3.*

### 22. Webhook Subscription System
**Status:** ✅ Implemented. `src/webhooks.rs` — persisted `WebhookStore` with HMAC-SHA256 signature verification and anti-replay (5min). CLI: `praxis webhook subscribe|list|unsubscribe`. Registered webhooks mapped to `POST /webhook/{name}` in dashboard.  
**Hermes equivalent:** `hermes webhook subscribe` — create named routes dynamically.  
**Why it matters:** Operators can wire Praxis into external systems without code changes.  ✅ *Done. Wave 6.*

### 23. Dry-Run / Replay Mode
**Status:** Mentioned as nice-to-have in roadmap (#13).  
**Hermes equivalent:** Not directly, but Hermes has session export/replay.  
**Why it matters:** Debugging agent behavior without side effects.  
**Effort:** Medium. Mock all tool execution, use recorded LLM responses.

### 24. Feature Flags / Gradual Rollout
**Status:** ✅ Implemented. `FeatureFlags` struct in `src/config/model.rs` with typed bool fields and `FeatureFlag` enum. `[features]` section in `praxis.toml`. Daemon logs enabled flags on reload.  
**Hermes equivalent:** Feature flag support for experimental features.  
**Why it matters:** Ship safely, test in production, roll back quickly.  ✅ *Done. Wave 5.*

### 25. Provider Auto-Failover
**Status:** Canary weights degrade but no automatic failover to backup providers.  
**Hermes equivalent:** Automatic failover between preferred and fallback providers.  
**Why it matters:** Agent availability when primary provider has an outage.  
**Effort:** Medium. Extend RouterBackend with fallback chain logic.

### 26. Honcho / Mem0 Memory Backends
**Status:** Only SQLite memory backend.  
**Hermes equivalent:** Pluggable memory backends (Honcho, Mem0).  
**Why it matters:** Scaling memory beyond local SQLite.  
**Effort:** High. Requires Rust clients for external memory services.

---

## Summary Table

| # | Feature | Status | Impact | Effort |
|---|---------|--------|--------|--------|
| 1 | Voice / STT / TTS | ❌ STUB | 🔴 Critical | Medium |
| 2 | Vision / Multi-Modal | ❌ Missing | 🔴 Critical | Medium |
| 3 | Config Hot-Reload | ✅ Done | 🔴 Critical | — |
| 4 | Code Execution (sandboxed) | ❌ Missing | 🟠 High | High |
| 5 | Browser Automation | ❌ Missing | 🟠 High | High |
| 6 | Persistent User Memory | ⚠️ Partial | 🟠 High | Medium |
| 7 | Credential Pooling | ✅ Done | 🟠 High | — |
| 8 | Cron Tool (agent-callable) | ✅ Done | 🟠 High | — |
| 9 | Clarify / Ask-User | ❌ Missing | 🟠 High | Low |
| 10 | Todo / Task Planning | ❌ Missing | 🟠 High | Low |
| 11 | Full Profile Isolation | ⚠️ Partial | 🟡 Nice | High |
| 12 | More Messaging Platforms | ⚠️ 3 of 15+ | 🟡 Nice | Variable |
| 13 | Plugin System | ❌ Missing | 🟡 Nice | Very High |
| 14 | ACP / IDE Integration | ⚠️ VSCode only | 🟡 Nice | Medium |
| 15 | Slash Commands | ❌ Missing | 🟡 Nice | Medium |
| 16 | Checkpoints / Rollback | ⚠️ Forensics only | 🟡 Nice | Medium |
| 17 | Worktree / Git Isolation | ❌ Missing | 🟡 Nice | Medium |
| 18 | Setup Wizard | ✅ Done | 🟡 Nice | — |
| 19 | Shell Completions | ✅ Done | 🟡 Nice | — |
| 20 | Image Generation | ✅ Done | 🟡 Nice | — |
| 21 | Usage Insights | ⚠️ Ledger only | 🟡 Nice | Low |
| 22 | Webhook Subscriptions | ✅ Done | 🟡 Nice | — |
| 23 | Dry-Run / Replay | ❌ Missing | 🟡 Nice | Medium |
| 24 | Feature Flags | ✅ Done | 🟡 Nice | — |
| 25 | Auto-Failover | ⚠️ Canary only | 🟡 Nice | Medium |
| 26 | Pluggable Memory Backends | ❌ Missing | 🟡 Nice | High |

---

### 26. Prompt Injection Protection
**Status:** ✅ Implemented. `src/context/injection.rs` scans identity files for 18 threat patterns before loading.  
**Hermes equivalent:** Scans all context files for prompt injection patterns before loading. Blocks files with threat patterns.  
**Why it matters:** Security. Prevents external files from hijacking agent behavior.  ✅ *Done. Wave 1.*

### 27. Progressive Context File Loading
**Status:** Not implemented. Praxis loads SOUL.md/IDENTITY.md once at session start.  
**Hermes equivalent:** Auto-discovers `.hermes.md` → `AGENTS.md` → `CLAUDE.md` → `.cursorrules` from CWD + git root. Progressively discovers sub-directory context files as the agent navigates.  
**Why it matters:** Per-subdirectory conventions in monorepos. Agent behavior adapts to which part of the project it's working in.  
**Effort:** Low-Medium. Extend existing context loader with directory-walking.

### 28. RL Training Pipeline (Atropos)
**Status:** Not implemented. Praxis has learning/opportunity mining but no model training.  
**Hermes equivalent:** Integrated GRPO RL training via Tinker-Atropos. Agent can list environments, configure training, launch runs, monitor WandB metrics, run inference tests.  
**Why it matters:** Self-improving agents that can fine-tune their own models.  
**Effort:** Very High. Requires full ML training infrastructure.

### 29. 6 Terminal Backends (beyond local)
**Status:** Local only.  
**Hermes equivalent:** local, Docker, SSH, Daytona (serverless), Singularity (containers), Modal (serverless GPU).  
**Why it matters:** Running the agent on remote machines, containers, or serverless infrastructure.  
**Effort:** High. Docker backend is feasible; others are major.

### 30. Plugin System (full — hooks, CLI commands, pip)
**Status:** Not implemented beyond a basic hooks system. Roadmap item #8.  
**Hermes equivalent:** Full plugin system with YAML manifests, pre/post tool call hooks, pre/post LLM call hooks, session start/end hooks, custom slash commands, custom CLI commands, message injection, bundled skills, pip-installable plugins, opt-in enablement, project-local plugins.  
**Why it matters:** Community ecosystem. Extensibility without core changes.  
**Effort:** Very High. Requires plugin host architecture.

### 31. Skills Hub & Registry
**Status:** Praxis has local SKILL.md files only. No registry, no sharing, no discovery.  
**Hermes equivalent:** agentskills.io skills hub with `hermes skills browse/search/install/inspect`. Skills security scanning. Skill taps (custom GitHub sources). Skills snapshots (export/import). Plugin-shipped skills (`plugin:skill` namespace).  
**Why it matters:** Community ecosystem. Operators sharing skills multiplies capability.  
**Effort:** High. Requires registry service + client.

### 32. Discord Voice Channels
**Status:** Not implemented. Discord integration is text-only (polling + webhooks).  
**Hermes equivalent:** Discord bot joins voice channels, listens to users speaking, speaks TTS replies via Opus codec. Requires Connect, Speak, Use Voice Activity permissions.  
**Why it matters:** Voice-first interaction on Discord — the most popular agent platform after Telegram.  
**Effort:** High. Requires Opus codec integration, voice activity detection, Discord voice gateway.

### 33. Migration Tooling
**Status:** Not implemented.  
**Hermes equivalent:** `hermes claw migrate` imports from OpenClaw (SOUL.md, memories, skills, command allowlists, messaging, API keys).  
**Why it matters:** Operator onboarding — migrating from Axonix or other agent frameworks.  
**Effort:** Low-Medium. Importers for Axonix config format.

### 34. Context File Prompt Injection Scanning
**Status:** Not implemented.  
**Hermes equivalent:** Scans AGENTS.md, SOUL.md, .cursorrules for prompt injection patterns before loading into context.  
**Why it matters:** Security.  
**Effort:** Low.

---

## Summary Table (Updated)

| # | Feature | Status | Impact | Effort |
|---|---------|--------|--------|--------|
| 1 | Voice / STT / TTS | ❌ STUB | 🔴 Critical | Medium |
| 2 | Vision / Multi-Modal | ❌ Missing | 🔴 Critical | Medium |
| 3 | Config Hot-Reload | ✅ Done | 🔴 Critical | — |
| 4 | Code Execution (sandboxed) | ❌ Missing | 🟠 High | High |
| 5 | Browser Automation | ❌ Missing | 🟠 High | High |
| 6 | Persistent User Memory | ✅ Done | 🟠 High | — |
| 7 | Credential Pooling | ✅ Done | 🟠 High | — |
| 8 | Cron Tool (agent-callable) | ✅ Done | 🟠 High | — |
| 9 | Clarify / Ask-User | ✅ Done | 🟠 High | — |
| 10 | Todo / Task Planning | ✅ Done | 🟠 High | — |
| 11 | Full Profile Isolation | ⚠️ Partial | 🟡 Nice | High |
| 12 | More Messaging Platforms | ⚠️ 3 of 15+ | 🟡 Nice | Variable |
| 13 | Plugin System | ❌ Missing | 🟡 Nice | Very High |
| 14 | ACP / IDE Integration | ⚠️ VSCode only | 🟡 Nice | Medium |
| 15 | Slash Commands | ❌ Missing | 🟡 Nice | Medium |
| 16 | Checkpoints / Rollback | ⚠️ Forensics only | 🟡 Nice | Medium |
| 17 | Worktree / Git Isolation | ❌ Missing | 🟡 Nice | Medium |
| 18 | Setup Wizard | ✅ Done | 🟡 Nice | — |
| 19 | Shell Completions | ✅ Done | 🟡 Nice | — |
| 20 | Image Generation | ✅ Done | 🟡 Nice | — |
| 21 | Usage Insights | ✅ Done | 🟡 Nice | — |
| 22 | Webhook Subscriptions | ✅ Done | 🟡 Nice | — |
| 23 | Dry-Run / Replay | ❌ Missing | 🟡 Nice | Medium |
| 24 | Feature Flags | ✅ Done | 🟡 Nice | — |
| 25 | Auto-Failover | ⚠️ Canary only | 🟡 Nice | Medium |
| 26 | Prompt Injection Protection | ✅ Done | 🟡 Nice | — |
| 27 | Progressive Context Files | ✅ Done | 🟡 Nice | — |
| 28 | Session Search | ✅ Done | 🟡 Nice | — |
| 29 | Session Timeline | ✅ Done | 🟡 Nice | — |
| 30 | Tool Use Policy Engine | ❌ Missing | 🟡 Nice | Medium |
| 31 | Skills Hub & Registry | ❌ Missing | 🟡 Nice | High |
| 32 | Discord Voice Channels | ❌ Missing | 🟡 Nice | High |
| 33 | Migration Tooling | ❌ Missing | 🟡 Nice | Low-Med |
| 34 | Context File Injection Scan | ✅ Done (dup of #26) | — | — |

---

## Recommended Priority Order

1. **Config Hot-Reload** — low-medium effort, operational necessity
2. **Credential Pooling** — medium effort, fixes rate limiting  
3. **Cron Tool (agent-callable)** — medium effort, enables self-scheduling
4. **Vision / Multi-Modal** — medium effort, unlocks photo/document workflows
5. **Voice / STT / TTS** — medium effort, unlocks mobile/ambient use
6. **Image Generation** — low effort, quick win
7. **Feature Flags** — low effort, safer shipping
8. **Setup Wizard** — low-medium effort, onboarding
9. **Browser Automation** — high effort but transformative
10. **Code Execution (sandboxed)** — high effort but unlocks coding agents
11. **Slash Commands** — medium effort, operator UX
12. **Skills Hub / Registry** — high effort but ecosystem-defining

The remaining items (browser, code execution, plugin system, full profiles, terminal backends, RL training) are major architectural undertakings best deferred until the foundation gaps are closed.

---

## Closure Log

### 2026-04-25 — Wave 1 (Quick Wins)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #9 | Clarify / Ask-User | ✅ DONE | `src/tools/clarify.rs` — publishes BusEvent, polls for operator response (5 min timeout) |
| #10 | Todo / Task Planning | ✅ DONE | `src/tools/todo.rs` — persisted `todo.json`, create/update/complete/cancel/list |
| #19 | Shell Completions | ✅ DONE | `praxis completions bash\|zsh\|fish\|elvish\|powershell` via `clap_complete` |
| #27/34 | Prompt Injection Protection | ✅ DONE | `src/context/injection.rs` — 18 patterns scanned on all identity/context file reads |
| #3 | Config Hot-Reload | ⏸️ DEFERRED | Needs `notify` crate + `Arc<RwLock<AppConfig>>` refactor of `PraxisRuntime` |

**Remaining open: 29 gaps** (was 33, 4 closed, 1 deferred)

### 2026-04-25 — Wave 2 (Context + Memory)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #28 | Progressive Context Files | ✅ DONE | `src/context/progressive.rs` — walks CWD→git root for `.praxis.md`, `AGENTS.md`, `CLAUDE.md`, `.cursorrules` |
| #6 | Persistent User Memory | ✅ DONE | `src/memory/user.rs` — `user_memory.json` key-value store with `memory` tool (upsert/search/forget/list) |

**Remaining open: 27 gaps** (was 29, 2 closed)

### 2026-04-25 — Wave 3 (Operator Experience)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #6 | Session Search | ✅ DONE | `src/storage/search.rs` + SQLite impl — `praxis sessions search <query> [--limit N]` searches outcome, action_summary, goal, task via LIKE |
| #21 | Usage Insights | ✅ DONE | `praxis insights [--days N]` — token totals, cost estimates, provider breakdown, memory counts, latest session |

**Remaining open: 25 gaps** (was 27, 2 closed)

### 2026-04-25 — Wave 4 (Config Hot-Reload)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #3 | Config Hot-Reload | ✅ DONE | `src/config/watcher.rs` — `notify` crate watches `praxis.toml`, daemon re-validates on change |

**Remaining open: 26 gaps** (was 27, 1 closed)

### 2026-04-25 — Wave 5 (Credential + Scheduling)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #7 | Credential Pooling | ✅ DONE | `src/backend/credential_pool.rs` — round-robin rotation, per-key 429 cooldown, `CREDENTIAL_POOLING` feature gate |
| #8 | Cron Tool (agent-callable) | ✅ DONE | `src/tools/cron.rs` — `ScheduledJobs` store, `every 30m`/`in 2h`/`hourly`/`daily`/`weekly` expressions |
| #24 | Feature Flags | ✅ DONE | `src/config/model.rs` — `FeatureFlags` struct, `[features]` section in `praxis.toml` |

**Remaining open: 23 gaps** (was 26, 3 closed)

### 2026-04-25 — Wave 6 (Setup + Webhooks + Image)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #18 | Setup Wizard | ✅ DONE | `src/cli/wizard.rs` — interactive `praxis init` via `dialoguer`, detects TTY and falls back to defaults in CI |
| #20 | Image Generation | ✅ DONE | `src/tools/image.rs` — DALL-E `/v1/images/generations`, credential pooling, size/n count params |
| #22 | Webhook Subscriptions | ✅ DONE | `src/webhooks.rs` — `WebhookStore` with HMAC-SHA256 + anti-replay, CLI: `praxis webhook subscribe\|list\|unsubscribe` |

**Remaining open: 20 gaps** (was 23, 3 closed)

### 2026-05-02 — Wave 7 (Vision + Failover + Voice)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #2 | Vision / Multi-Modal | ✅ DONE | `src/backend/mod.rs` — `InputContent` enum (Text/Blocks), `ContentBlock` (Text/ImageUrl). OpenAI, Claude, Ollama backends handle multi-modal content blocks. `src/tools/vision.rs` — vision tool with URL + local file via base64 data URLs. |
| #1 | Voice / STT / TTS | ⚠️ STUB → PLACEHOLDER | `src/tools/voice.rs` — voice tool with `speech_to_text` and `text_to_speech` commands. Placeholder implementations pending whisper-rs (needs libclang) and edge-tts HTTP API. |
| #25 | Provider Auto-Failover | ✅ DONE | `src/backend/health.rs` — `ProviderHealthTracker` with consecutive failure counting (3 = unhealthy), 60s persistence debounce. Integrated into `execute_routes` to skip unhealthy providers. |

**Remaining open: 17 gaps** (was 20, 3 closed)

### 2026-05-02 — Wave 8 (Chat + ACP + Checkpoints)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #15 | Slash Commands / Interactive Session | ✅ DONE | `src/cli/chat.rs` — `praxis chat` REPL with 14 slash commands: /help, /new, /model, /verbose, /compact, /retry, /undo, /history, /skills, /tools, /status, /save, /export, /quit. Conversation history with undo support. |
| #14 | ACP / IDE Integration | ✅ DONE | `src/cli/acp.rs` — `praxis acp` JSON-RPC 2.0 over stdio. Methods: initialize, tools/list, tools/call, chat, context/get, status. Compatible with Claude Code, Cursor, Windsurf. |
| #16 | Checkpoints / Rollback | ✅ DONE | `src/cli/checkpoint.rs` — `praxis checkpoint [--label]`, `praxis rollback <id>`, `praxis checkpoints`. Copy-on-write with auto pre-rollback save. Keeps last 20. |

**Remaining open: 14 gaps** (was 17, 3 closed)

### 2026-05-02 — Wave 9 (Migration + Worktree + Dry-Run + Profiles)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #33 | Migration Tooling | ✅ DONE | `src/cli/migrate.rs` — `praxis migrate <path> [--dry-run]`. Imports SOUL.md, IDENTITY.md, GOALS.md, AGENTS.md, skills/, tools/, user_memory.json, hooks.toml, telegram_state.json from Axonix data dirs. Skips existing files, reports imported/skipped/errors. |
| #17 | Worktree / Git Isolation | ✅ DONE | `src/cli/worktree.rs` — `praxis worktree create|list|remove|merge`. Creates git worktrees with `praxis/<name>` branches, supports merge-back and branch cleanup. |
| #23 | Dry-Run / Replay | ✅ DONE | `src/cli/dryrun.rs` — `praxis plan create|list|show|dry-run|remove`. ExecutionPlan with sequential steps, parameter validation, destructive command detection, JSON persistence. |
| #11 | Profile Isolation | ✅ DONE | `src/cli/profile.rs` — `praxis profile create|switch|list|remove|show`. Each profile gets isolated tools/, skills/, memory/, sessions/ subdirectories. |

**Remaining open: 10 gaps** (was 14, 4 closed)
