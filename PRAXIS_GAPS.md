# Praxis Gaps — What Praxis Is Missing

**Generated:** 2026-05-02 by Scout  
**Sources:** Praxis codebase (all docs), Hermes Agent v0.7–v0.12 release notes (full RELEASE_v0.12.0.md + GitHub tags), OpenClaw docs, ecosystem benchmarking  
**Hermes baseline:** v0.12.0 "Curator" (2026-04-30) — 1,096 commits, 550 PRs since v0.11  
**Praxis baseline:** Wave 9 (2026-05-02)

---

## Executive Summary

The original GAP_ANALYSIS_HERMES.md (2026-04-25) identified 34 gaps. Praxis closed 24. Since then, **Hermes shipped 5 major releases in April alone** (v0.8–v0.12), roughly one every 6 days. This document captures everything.

**Total: 65 gaps** — 10 original still open + 33 new from Hermes v0.8–v0.12 + 13 OpenClaw-exclusive + 9 ecosystem.

---

## 🔴 CRITICAL — Must Have (7)

### 1. Browser Automation ❌
**Hermes:** 12 browser tools (navigate, snapshot, vision, click, type, press, scroll, back, console, get_images, cdp, dialog). Backends: Browserbase, Browser Use, Camofox anti-detect, local Chrome CDP, local Chromium. Zero-API-key via Nous Tool Gateway. v0.12: CDP supervisor with dialog detection + cross-origin iframe eval.
**OpenClaw:** Isolated browser + signed-in Chrome via Chrome MCP. Playwright-backed.
**Praxis:** ❌ Nothing.  
**Effort:** High.

### 2. Code Execution (Sandboxed Python) ❌
**Hermes:** `execute_code` — Python with programmatic access to all tools. v0.12: **Vercel Sandbox backend** for serverless execution.  
**Praxis:** ❌ Raw `/bin/bash -c`. Phase 2B.  
**Effort:** High.

### 3. Plugin System ❌
**Hermes:** v0.11: Plugins can block tools, rewrite results, transform terminal output, register slash commands, dispatch tools, add dashboard tabs, ship image_gen backends. v0.12: **Pluggable gateway platforms** (platforms as plugins), **bundled plugins** (Spotify, Google Meet, Langfuse, achievements), new hooks (`pre_gateway_dispatch`, `pre_approval_request`, `post_approval_response`), **Podman support** (v0.11).  
**OpenClaw:** 4 plugin types (channel, memory, tool, provider).  
**Praxis:** ❌ Rust recompile required.  
**Effort:** Very High.

### 4. Skills Hub & Registry ❌
**Hermes:** agentskills.io. 104+ bundled skills. v0.12: ComfyUI v5 built-in, TouchDesigner-MCP bundled, Humanizer, claude-design, airtable, pretext, spike, sketch, llm-wiki. Direct URL install. `/reload-skills`.  
**OpenClaw:** ClawHub — 100+ bundles.  
**Praxis:** ❌ Local SKILL.md only.  
**Effort:** High.

### 5. Plugin Surface — Block/Rewrite/Intercept (v0.11) ❌
Plugins as middleware — block tool execution, rewrite results, transform terminal output.  
**Praxis:** ❌ HookRunner aborts phases but can't rewrite tool output.  
**Effort:** Very High. Depends on #3.

### 6. Autonomous Curator (v0.12) ❌
**Hermes's headline v0.12 feature.** Background agent that grades, prunes, and consolidates the agent's OWN skill library on a 7-day cycle. Per-run reports. `hermes curator status` ranks skills by usage. Rubric-based grading. Scoped toolsets (memory + skills only).  
**Praxis:** ❌ Evolution proposals touch config/identity only. No skill maintenance.  
**Effort:** Medium-High.

### 7. One-Shot Mode `hermes -z` (v0.12) ❌
Non-interactive fire-and-forget with FULL tool access. `--model`/`--provider` flags.  
**Praxis:** ⚠️ `praxis ask` exists but is read-only. No tool-executing one-shot.  
**Effort:** Low.

---

## 🟠 HIGH — Major Capability Gaps (22)

### 8. Messaging Platforms ⚠️
**Hermes: 19 platforms** (+ Teams v0.12, + Yuanbao v0.12, + QQBot v0.11). Gateway is now a **plugin host for platforms** (v0.12).  
**OpenClaw:** 23+.  
**Praxis:** ⚠️ 3.  
**Effort:** Variable.

### 9. Voice / TTS ⚠️
**Hermes:** STT + TTS. v0.12: **Pluggable TTS registry** + **Piper** native local TTS (free). Centralized audio routing with FLAC.  
**Praxis:** ⚠️ Placeholder stubs.  
**Effort:** Medium.

### 10. Discord Voice Channels ❌
Bot joins voice, speaks TTS via Opus.  
**Effort:** High. Depends on #9.

### 11. Pluggable Memory Backends ❌
Honcho, OpenViking, Mem0, Hindsight, Holographic, RetainDB, ByteRover. v0.12: Curator cleanly shuts down memory providers.  
**Effort:** High.

### 12. Tool Use Policy Engine ❌
Per-platform toggles, composite toolsets, platform presets. v0.12: **Slack `channel_skill_bindings`** + **`strict_mention`**. **Telegram chat allowlists**. v0.11: **Per-channel ephemeral prompts** (Discord, Telegram, Slack, Mattermost), **`require_mention` + `allowed_users` gating**.  
**Praxis:** ⚠️ Per-tool levels only.  
**Effort:** Medium.

### 13. Canvas / A2UI ❌
**OpenClaw exclusive.** Agent-driven visual workspace with interactive HTML/A2UI.  
**Effort:** Very High.

### 14. Multi-Agent Workspace Isolation ❌
**OpenClaw exclusive.** Per-channel/group agent routing to isolated workspaces.  
**Effort:** High.

### 15. Voice Wake / Talk Mode ❌
**OpenClaw exclusive.** "Hey OpenClaw" wake word on macOS/iOS/Android.  
**Effort:** Very High.

### 16. `/steer` Mid-Run Nudges (v0.11) ❌
Inject notes agent sees after next tool call.  
**Effort:** Low-Med.

### 17. Shell Hooks (v0.11) ❌
Wire shell scripts as lifecycle hooks, no Python required.  
**Effort:** Low.

### 18. Webhook Direct-Delivery (v0.11) ❌
Forward webhooks to chat bypassing agent/LLM.  
**Effort:** Low.

### 19. Dashboard Plugin System (v0.11) + Chat Tab (v0.12) ❌
v0.11: 3rd-party tabs/widgets. v0.12: **Dashboard Chat tab** (xterm.js + JSON-RPC sidecar — full web-based terminal to agent). **Models tab** with per-model analytics. **Page-scoped plugin slots**. Configure main + auxiliary models from dashboard.  
**Praxis:** ⚠️ Static SPA dashboard.  
**Effort:** Medium-High.

### 20. Transport ABC (v0.11) ❌
Pluggable provider transport layer. Clean architecture for adding providers.  
**Effort:** Medium.

### 21. `hermes fallback` Command (v0.12) ❌
CLI for managing fallback provider chains.  
**Praxis:** ⚠️ Auto-failover (Wave 7) but no CLI for managing fallback chains.  
**Effort:** Low-Med.

### 22. Auto-Backup Before Update (v0.12) ❌
Auto-backup HERMES_HOME before `hermes update` (opt-in). `hermes update --check` preflight.  
**Praxis:** ❌ Watchdog binary update exists but no auto-backup.  
**Effort:** Low.

### 23. Fast Mode `/fast` (v0.9) ❌
Priority processing. v0.12: whitelist broadened to all OpenAI + Anthropic models.  
**Effort:** Low.

### 24. `notify_on_complete` (v0.8) ❌
Auto-notification when background tasks finish.  
**Effort:** Low-Med.

### 25. Approval Buttons (v0.8) ❌
Platform-native buttons on Slack/Telegram.  
**Effort:** Low-Med.

### 26. Native Spotify (v0.12) ❌
7 tools (play, search, queue, playlists, devices) with PKCE OAuth.  
**Effort:** Medium.

### 27. Native Google Meet (v0.12) ❌
Join calls, transcribe, speak, follow up.  
**Effort:** Medium.

### 28. Pluggable Gateway Platforms (v0.12) ❌
Gateway is a plugin host. Any platform can ship as plugin. Teams is first.  
**Effort:** High. Depends on #3.

### 29. Live Model Switching `/model` (v0.8) ❌
Switch models mid-session without restart.  
**Effort:** Medium.

---

## 🟡 NICE-TO-HAVE (36)

### 30. Self-Improvement Loop Upgrades (v0.12)
Rubric-based grading, active-update bias, runtime inheritance, scoped toolsets, clean context.  
**Effort:** Medium.

### 31. ComfyUI v5 + TouchDesigner-MCP + Humanizer + claude-design + pretext + spike + sketch + airtable + llm-wiki (v0.11–v0.12)
Creative and productivity skill ecosystem.  
**Effort:** Low-Med (curation, not engineering).

### 32. Langfuse Observability + Hermes Achievements (v0.12)
LLM tracing plugin + gamification.  
**Effort:** Medium.

### 33. Native Multi-Image Sending (v0.12)
Across 6 platforms.  
**Effort:** Low-Med.

### 34. Centralized Audio Routing + FLAC (v0.12)
**Effort:** Medium. Depends on #9.

### 35. Remote Model Catalog (v0.12)
Auto-discovery of new models from OpenRouter + Nous Portal manifests.  
**Effort:** Low-Med.

### 36. Configurable Prompt Cache TTL (v0.12)
5-min default, 1-hour opt-in.  
**Effort:** Low.

### 37. Vercel Sandbox Backend (v0.12)
Serverless execution for code/terminal.  
**Effort:** High.

### 38. 9 New Inference Providers (v0.11–v0.12)
GMI Cloud, Azure AI Foundry, MiniMax OAuth, Tencent Tokenhub, LM Studio (upgraded), NVIDIA NIM, Arcee AI, Step Plan, Gemini CLI OAuth, Vercel ai-gateway, AWS Bedrock. GPT-5.5 via Codex. DeepSeek-v4-pro/flash, Qwen 3.6 Plus, GLM-5V-Turbo.  
**Effort:** Low-Med per provider.

### 39. Cron `workdir` + `context_from` Chaining (v0.12) + `web_search limit` (v0.12)
**Effort:** Low-Med.

### 40. TUI Enhancements (v0.11–v0.12)
Ink/React rewrite, ~57% cold start cut (v0.12), LaTeX rendering (v0.12), `/reload` .env hot-reload, auto-resume, session deletion from picker, `/mouse` toggle, pluggable busy-indicator styles, `hermes -z` one-shot.  
**Effort:** High (ground-up).

### 41. Per-Channel Ephemeral Prompts (v0.11)
Channel-specific prompts that don't persist across sessions.  
**Effort:** Low-Med.

### 42. `require_mention` + `allowed_users` Gating (v0.11)
Parity across Discord/Telegram/Slack gatekeeping.  
**Effort:** Low.

### 43. Inline Image Inputs on API (v0.11)
`/v1/chat/completions` and `/v1/responses` accept inline images.  
**Effort:** Low-Med.

### 44. GPT Image 2 (v0.11) + Pluggable image_gen Backends (v0.11)
OpenAI provider for image generation.  
**Effort:** Low-Med.

### 45. Podman Support (v0.11)
Rootless container entrypoint alongside Docker.  
**Effort:** Low-Med.

### 46. Nix Support (v0.12)
Nix/NixOS package + plugin install.  
**Effort:** Low.

### 47. Secret Redaction Off by Default (v0.12)
Security design decision — prevents patch corruption. Opt-in.  
**Effort:** Low.

### 48. Hardline Blocklist + `[IMPORTANT:` Markers (v0.12)
Unrecoverable command blocklist. Azure content filter dodge.  
**Effort:** Low.

### 49. Dashboard Models Tab (v0.12)
Rich per-model analytics.  
**Effort:** Low-Med.

### 50. Orchestrator Role + max_spawn_depth (v0.11)
Cross-agent file state coordination.  
**Effort:** Medium.

### 51. Inactivity-Based Timeouts (v0.8)
Track tool activity, not wall-clock.  
**Effort:** Low.

### 52–65. OpenClaw + Ecosystem Gaps
Session Tools, Named Persistent Sessions, Device Nodes, Control UI, WebChat, WebSocket Control Plane, DM Scoping, Tool Profiles, WASM Sandbox, Vector/Embedding Memory, Secret Zeroization, Merkle Audit Trail, Rule-Based Routing, Autonomy Levels, CodeAct Mode, Mission System, Hands System, Context References, Batch Processing, API Server (OpenAI-compat), OpenClaw Migration.  
**Effort:** Low to Very High.

---

## Hermes Release Cadence (April 2026)

| Version | Date | Days gap | Headline |
|---------|------|----------|----------|
| v0.7 | Apr 3 | — | Memory providers, credential pools |
| v0.8 | Apr 8 | 5 | Live model switching, notify_on_complete, approval buttons |
| v0.9 | Apr 13 | 5 | Local dashboard, Fast Mode, iMessage/WeChat/Termux, 16 platforms |
| v0.10 | Apr 16 | 3 | Nous Tool Gateway (zero-API-key tools) |
| v0.11 | Apr 23 | 7 | Ink TUI, Transport ABC, /steer, shell hooks, orchestrator, 104 skills, GPT-5.5, QQBot, 5 providers |
| v0.12 | Apr 30 | 7 | **Curator**, Spotify, Google Meet, Teams, Yuanbao, ComfyUI v5, Piper TTS, one-shot, pluggable platforms, 4 providers |

---

## What Praxis Still Has That Neither Competitor Has

| Feature | Praxis | Hermes | OpenClaw |
|---------|--------|--------|----------|
| 4-phase loop (Orient→Decide→Act→Reflect) | ✅ | ❌ | ❌ |
| Irreplaceability Score (4-dim) | ✅ | ❌ | ❌ |
| Self-Evolution Proposals (auto-gen) | ✅ | ❌ | ❌ |
| Argus Drift/Pattern Detection | ✅ | ❌ | ❌ |
| Model Canary Health Probes | ✅ | ❌ | ❌ |
| Hot/Cold/Link Memory Tiers + FTS5 | ✅ | ❌ | ❌ |
| Speculative Execution | ✅ | ❌ | ❌ |
| Git-Backed State Sync (auto-commit) | ✅ | ❌ | ❌ |
| AES-256-GCM At-Rest Encryption | ✅ | ❌ | ❌ |
| Context Budget System | ✅ | ❌ | ❌ |
| Adaptive Scheduling (Quiet Hours) | ✅ | ❌ | ❌ |
| Output Quality Gates (deterministic) | ✅ | ❌ | ❌ |
| Boundary Review System | ✅ | ❌ | ❌ |
| Rust — Single Binary, Zero Runtime Deps | ✅ | ❌ | ❌ |

---

## Recommended Priority

### Immediate
1. One-Shot Mode (#7) — low effort
2. Shell Hooks (#17) — low effort
3. Webhook Direct-Delivery (#18) — low effort
4. `notify_on_complete` (#24) — low-med
5. Approval Buttons (#25) — low-med
6. Voice/TTS (#9) — placeholder exists

### Short-Term
7. Browser Automation (#1)
8. Code Execution Sandbox (#2)
9. Tool Use Policy Engine (#12)
10. Transport ABC (#20)
11. Live Model Switching (#29)
12. `/steer` (#16)

### Medium-Term
13. Plugin System (#3, #5)
14. Autonomous Curator (#6)
15. Skills Hub (#4)
16. Pluggable Gateway Platforms (#28)
17. Multi-Agent Workspace (#14)
18. More Platforms (#8)

### Long-Term
19. Canvas/A2UI (#13)
20. Voice Wake (#15)
21. Everything else
