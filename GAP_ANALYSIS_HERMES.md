# Praxis vs Hermes Agent — Gap Analysis

*Generated 2026-04-25 by Drey. Comparison of Praxis (`/mnt/docker/code/praxis`) against the Hermes Agent feature set.*

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
**Status:** Not implemented. Daemon loads `praxis.toml` once at startup. Roadmap item #5.  
**Hermes equivalent:** Config changes take effect on gateway `/restart` or CLI relaunch. Hermes also has `hermes config set KEY VAL` for programmatic changes.  
**Why it matters:** An always-on daemon shouldn't require a restart to change backend, budget, or security settings.  
**Effort:** Low-Medium. `notify` crate to watch config file, re-validate, atomically swap.

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
**Status:** Partial. Praxis has hot/cold agent memory but no persistent *user profile* memory (operator preferences, facts, corrections). Memory is agent-scoped, not operator-scoped.  
**Hermes equivalent:** `memory` tool with `user` and `memory` targets. Survives across sessions. Pluggable backends (built-in, Honcho, Mem0). `session_search` for recalling past conversations.  
**Why it matters:** "Remember I prefer X" should persist forever. Without this, the agent forgets operator preferences between sessions.  
**Effort:** Medium. Build on existing hot/cold memory system but add user-profile persistence and session search.

### 7. Credential Pooling / API Key Rotation
**Status:** Not implemented. Single API key per provider. Roadmap references "credential pools" from GoClaw but no implementation.  
**Hermes equivalent:** `hermes auth add` for credential pools. Automatic rotation across multiple keys per provider to avoid rate limits.  
**Why it matters:** Rate limits are the #1 cause of agent downtime. Pooling is the standard solution.  
**Effort:** Medium. Extend ProviderSettings to support arrays of credentials, add round-robin/weighted selection.

### 8. Cron Job Management as a Tool
**Status:** Cron scheduling exists in the watchdog binary, but the agent cannot create/edit/pause/run cron jobs via tool calls during a session.  
**Hermes equivalent:** `cronjob` tool — create, list, update, pause, resume, remove, run cron jobs from within agent context. Jobs deliver results to messaging platforms.  
**Why it matters:** Self-directed agents should be able to schedule their own recurring tasks.  
**Effort:** Medium. Expose watchdog cron operations as tools in the tool registry.

### 9. Clarify / Ask-User Tool
**Status:** Not implemented.  
**Hermes equivalent:** `clarify` tool — ask the operator clarifying questions with multiple-choice or free-form options.  
**Why it matters:** Agents hit ambiguity. Without clarify, they guess or fail silently.  
**Effort:** Low. Simple tool that writes a question to the messaging bus and blocks until operator responds.

### 10. Todo / In-Session Task Planning
**Status:** Not implemented. Goals exist at the file level but there's no per-session task decomposition.  
**Hermes equivalent:** `todo` tool — create/update/complete task items within a session. Visual task tracking.  
**Why it matters:** Complex tasks need decomposition. Agents that can't plan their own work are less reliable.  
**Effort:** Low. JSON-based task list in session state, surfaced in dashboard.

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
**Status:** `praxis init` is simple, no interactive wizard.  
**Hermes equivalent:** `hermes setup` — interactive wizard for model, terminal, gateway, tools, agent configuration.  
**Why it matters:** Onboarding. New operators shouldn't need to read TOML docs to get started.  
**Effort:** Low-Medium. Interactive prompts for key config values.

### 19. Shell Completions
**Status:** Not implemented.  
**Hermes equivalent:** `hermes completion bash|zsh` — generates shell completion scripts.  
**Why it matters:** Operator quality of life.  
**Effort:** Low. clap can auto-generate completions.

### 20. Image Generation
**Status:** Not implemented.  
**Hermes equivalent:** `image_generate` tool — AI image generation from text prompts.  
**Why it matters:** Creative/presentation use cases.  
**Effort:** Low. HTTP call to DALL-E/Stable Diffusion API.

### 21. Usage Analytics / Insights
**Status:** Token ledger exists but no analytics command.  
**Hermes equivalent:** `hermes insights [--days N]` — cost, token usage, session patterns.  
**Why it matters:** Operators want to know what their agent costs and how it spends its time.  
**Effort:** Low. Query token_ledger table, render summary.

### 22. Webhook Subscription System
**Status:** Webhook endpoints exist in dashboard (`/webhook/discord`, `/webhook/slack`) but no dynamic subscription system.  
**Hermes equivalent:** `hermes webhook subscribe` — create named routes dynamically.  
**Why it matters:** Operators can wire Praxis into external systems without code changes.  
**Effort:** Low-Medium. Dynamic route registration + persistence.

### 23. Dry-Run / Replay Mode
**Status:** Mentioned as nice-to-have in roadmap (#13).  
**Hermes equivalent:** Not directly, but Hermes has session export/replay.  
**Why it matters:** Debugging agent behavior without side effects.  
**Effort:** Medium. Mock all tool execution, use recorded LLM responses.

### 24. Feature Flags / Gradual Rollout
**Status:** Not implemented. Roadmap item #6.  
**Hermes equivalent:** Feature flag support for experimental features.  
**Why it matters:** Ship safely, test in production, roll back quickly.  
**Effort:** Low. TOML feature flags in config.

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
| 3 | Config Hot-Reload | ❌ Missing | 🔴 Critical | Low-Med |
| 4 | Code Execution (sandboxed) | ❌ Missing | 🟠 High | High |
| 5 | Browser Automation | ❌ Missing | 🟠 High | High |
| 6 | Persistent User Memory | ⚠️ Partial | 🟠 High | Medium |
| 7 | Credential Pooling | ❌ Missing | 🟠 High | Medium |
| 8 | Cron Tool (agent-callable) | ❌ Missing | 🟠 High | Medium |
| 9 | Clarify / Ask-User | ❌ Missing | 🟠 High | Low |
| 10 | Todo / Task Planning | ❌ Missing | 🟠 High | Low |
| 11 | Full Profile Isolation | ⚠️ Partial | 🟡 Nice | High |
| 12 | More Messaging Platforms | ⚠️ 3 of 15+ | 🟡 Nice | Variable |
| 13 | Plugin System | ❌ Missing | 🟡 Nice | Very High |
| 14 | ACP / IDE Integration | ⚠️ VSCode only | 🟡 Nice | Medium |
| 15 | Slash Commands | ❌ Missing | 🟡 Nice | Medium |
| 16 | Checkpoints / Rollback | ⚠️ Forensics only | 🟡 Nice | Medium |
| 17 | Worktree / Git Isolation | ❌ Missing | 🟡 Nice | Medium |
| 18 | Setup Wizard | ❌ Missing | 🟡 Nice | Low-Med |
| 19 | Shell Completions | ❌ Missing | 🟡 Nice | Low |
| 20 | Image Generation | ❌ Missing | 🟡 Nice | Low |
| 21 | Usage Insights | ⚠️ Ledger only | 🟡 Nice | Low |
| 22 | Webhook Subscriptions | ⚠️ Static only | 🟡 Nice | Low-Med |
| 23 | Dry-Run / Replay | ❌ Missing | 🟡 Nice | Medium |
| 24 | Feature Flags | ❌ Missing | 🟡 Nice | Low |
| 25 | Auto-Failover | ⚠️ Canary only | 🟡 Nice | Medium |
| 26 | Pluggable Memory Backends | ❌ Missing | 🟡 Nice | High |

---

### 26. Prompt Injection Protection
**Status:** Not implemented.  
**Hermes equivalent:** Scans all context files (AGENTS.md, SOUL.md, .cursorrules) for prompt injection patterns before loading. Blocks files with threat patterns.  
**Why it matters:** Security. Prevents external files from hijacking agent behavior.  
**Effort:** Low. Regex-based scanning of loaded context files.

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
| 3 | Config Hot-Reload | ❌ Missing | 🔴 Critical | Low-Med |
| 4 | Code Execution (sandboxed) | ❌ Missing | 🟠 High | High |
| 5 | Browser Automation | ❌ Missing | 🟠 High | High |
| 6 | Persistent User Memory | ⚠️ Partial | 🟠 High | Medium |
| 7 | Credential Pooling | ❌ Missing | 🟠 High | Medium |
| 8 | Cron Tool (agent-callable) | ❌ Missing | 🟠 High | Medium |
| 9 | Clarify / Ask-User | ❌ Missing | 🟠 High | Low |
| 10 | Todo / Task Planning | ❌ Missing | 🟠 High | Low |
| 11 | Full Profile Isolation | ⚠️ Partial | 🟡 Nice | High |
| 12 | More Messaging Platforms | ⚠️ 3 of 15+ | 🟡 Nice | Variable |
| 13 | Plugin System (full) | ❌ Missing | 🟡 Nice | Very High |
| 14 | ACP / IDE Integration | ⚠️ VSCode only | 🟡 Nice | Medium |
| 15 | Slash Commands | ❌ Missing | 🟡 Nice | Medium |
| 16 | Checkpoints / Rollback | ⚠️ Forensics only | 🟡 Nice | Medium |
| 17 | Worktree / Git Isolation | ❌ Missing | 🟡 Nice | Medium |
| 18 | Setup Wizard | ❌ Missing | 🟡 Nice | Low-Med |
| 19 | Shell Completions | ❌ Missing | 🟡 Nice | Low |
| 20 | Image Generation | ❌ Missing | 🟡 Nice | Low |
| 21 | Usage Insights | ⚠️ Ledger only | 🟡 Nice | Low |
| 22 | Webhook Subscriptions | ⚠️ Static only | 🟡 Nice | Low-Med |
| 23 | Dry-Run / Replay | ❌ Missing | 🟡 Nice | Medium |
| 24 | Feature Flags | ❌ Missing | 🟡 Nice | Low |
| 25 | Auto-Failover | ⚠️ Canary only | 🟡 Nice | Medium |
| 26 | Pluggable Memory Backends | ❌ Missing | 🟡 Nice | High |
| 27 | Prompt Injection Protection | ❌ Missing | 🟠 High | Low |
| 28 | Progressive Context Files | ❌ Missing | 🟡 Nice | Low-Med |
| 29 | RL Training Pipeline | ❌ Missing | 🟡 Nice | Very High |
| 30 | 6 Terminal Backends | ⚠️ Local only | 🟡 Nice | High |
| 31 | Skills Hub / Registry | ❌ Missing | 🟠 High | High |
| 32 | Discord Voice Channels | ❌ Missing | 🟡 Nice | High |
| 33 | Migration Tooling | ❌ Missing | 🟡 Nice | Low-Med |

---

## Recommended Priority Order

1. **Clarify / Ask-User** — lowest effort, highest operator experience impact
2. **Todo / Task Planning** — low effort, makes agent more capable
3. **Prompt Injection Protection** — low effort, security essential
4. **Config Hot-Reload** — low-medium effort, operational necessity
5. **Persistent User Memory + Session Search** — medium effort, core to "knowing you"
6. **Vision / Multi-Modal** — medium effort, unlocks photo/document workflows
7. **Credential Pooling** — medium effort, fixes rate limiting
8. **Cron Tool (agent-callable)** — medium effort, enables self-scheduling
9. **Progressive Context Files** — low-medium effort, better monorepo support
10. **Voice / STT / TTS** — medium effort, unlocks mobile/ambient use
11. **Shell Completions** — trivial, quick operator win
12. **Skills Hub / Registry** — high effort but ecosystem-defining

The remaining items (browser, code execution, plugin system, full profiles, terminal backends, RL training) are major architectural undertakings best deferred until the foundation gaps are closed.

---

## Closure Log

### 2026-04-25 — Wave 1 (Quick Wins)

| Gap | Feature | Status | Commit |
|-----|---------|--------|--------|
| #9 | Clarify / Ask-User | ✅ DONE | `src/tools/clarify.rs` — publishes BusEvent, polls for operator response (5 min timeout) |
| #10 | Todo / Task Planning | ✅ DONE | `src/tools/todo.rs` — persisted `todo.json`, create/update/complete/cancel/list |
| #19 | Shell Completions | ✅ DONE | `praxis completions bash|zsh|fish|elvish|powershell` via `clap_complete` |
| #27/34 | Prompt Injection Protection | ✅ DONE | `src/context/injection.rs` — 18 patterns scanned on all identity/context file reads |
| #3 | Config Hot-Reload | ⏸️ DEFERRED | Needs `notify` crate + `Arc<RwLock<AppConfig>>` refactor of `PraxisRuntime` |

**Remaining open: 29 gaps** (was 33, 4 closed, 1 deferred)
