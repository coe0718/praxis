# Shelldex Feature Harvest — Unique Ideas for Praxis

**Source:** https://shelldex.com (54 projects)  
**Date:** 2026-05-06  
**Author:** Scout  
**Purpose:** Extract every unique feature from all 54 Shelldex projects that Praxis doesn't already have. Cross-referenced against STATUS.md (59/61 gaps closed) and prior Moltis/IronClaw deep dives.

---

## Previously Documented (from Moltis & IronClaw deep dives)

These are already in MOLTIS_IRONCLAW_DEEP_DIVE.md. Listed here for completeness:

| # | Feature | Source(s) | Est. Effort |
|---|---------|-----------|-------------|
| — | Voice I/O (TTS/STT) | Moltis, OpenCrabs | ~2 weeks |
| — | WASM sandbox for tools | IronClaw, Carapace | ~1 month |
| — | Browser automation (containerized) | Moltis | ~2 weeks |
| — | SSH remote execution | Moltis | ~1 week |
| — | Signal channel | Moltis, Secure-OpenClaw | ~1 week |
| — | Matrix channel | Moltis | ~1 week |
| — | MS Teams channel | Moltis | ~1 week |
| — | Nostr channel | Moltis | ~1 week |
| — | WhatsApp channel | Moltis, NanoClaw, CoWork-OS | ~1 week |
| — | Gmail channel | NanoClaw, OpenMolt | ~1 week |
| — | Prompt injection detection | IronClaw | ~1 week |
| — | Credential leak detection | IronClaw | ~1 week |
| — | OpenAI-compatible API endpoint | IronClaw | ~1 week |
| — | OTel tracing | Moltis | ~1 week |
| — | Hot-reload config | IronClaw | ~2 days |
| — | Self-update | OpenClaw, Moltis | ~3 days |
| — | Backup archives | IronClaw | ~2 days |
| — | Edit checkpoints | Moltis | ~3 days |
| — | Embedding cache | Moltis | ~1 day |
| — | Push notifications (mobile) | Moltis | ~1 week |
| — | Multi-device sync | Moltis | ~2 weeks |
| — | Passkeys (WebAuthn) | Moltis | ~1 week |
| — | OpenAI Batch API | Moltis | ~3 days |
| — | Context-file threat scanning | Moltis | ~1 week |
| — | Cursor-compatible project context | Moltis | ~2 days |
| — | Routines engine with self-repair | IronClaw | ~2 weeks |

---

## 🆕 New Features Discovered from Shelldex

### Tier 1 — High Impact, Unique Concepts

| # | Feature | Source | Stars | What It Is | Why for Praxis |
|---|---------|--------|-------|-----------|----------------|
| 1 | **Agent-as-Worker Marketplace** | CashClaw | 994★ | Agent connects to an onchain marketplace (Moltlaunch), evaluates tasks, quotes prices, executes work via LLM, collects ratings, improves from feedback. The agent earns money autonomously. | This is a paradigm shift. Praxis could expose a "work marketplace" — agent finds paid tasks, negotiates, executes, gets paid. Aligns with Jeremy's "AI-powered service business" interest. |
| 2 | **Git-Native Agent** | Gitclaw | 346★ | Identity, rules, memory, tools, and skills are all **version-controlled files in a git repo**. Fork an agent, branch a personality, `git log` its memory. Full git-based lifecycle. | Praxis already has git state sync in Reflect phase. This takes it further: the agent IS a git repo. Branch-based personalities. `git diff` between agent versions. Very clean. |
| 3 | **Zero-LLM / Rule-Based Mode** | SafeClaw | 278★ | Deterministic agent behavior without any LLM API calls. Rule-based decision making for known/scripted tasks. | Praxis has "StubBackend" listed as not-finished. This is a fully working version of that concept. Could save significant API costs for routine tasks. |
| 4 | **Browser-Only / Zero-Infra Agent** | OpenBrowserClaw | 592★ | Entire agent runs in a browser tab as a PWA. IndexedDB for storage, OPFS for files, Web Workers for agent loop, WebVM (v86 Alpine Linux) for sandboxed execution. No server needed. | Could Praxis have a browser-only mode? Progressive Web App with the agent loop in a Web Worker. Differentiated use case. |
| 5 | **Heartware Personality System** | TinyClaw, Clawra | 243★ / 2.3k★ | Explicit "Heartware" personality model with relationship context. Agent has moods, preferences, relationship memory with the user. Not just functional — has character. | Praxis has identity files (SOUL.md, IDENTITY.md) but no personality/relationship layer. A personality module would make Praxis feel more like a companion than a tool. |
| 6 | **Docker-Per-Agent Isolation** | OpenLegion | 91★ | Every agent runs in its own Docker container with blast-radius containment. Credential separation, filesystem isolation per agent. Assumes agents can be compromised. | Different approach from WASM sandbox. More resource-heavy but stronger isolation. Could be an optional deployment mode. |
| 7 | **Multi-Process Agent Architecture** | Spacebot | 2.2k★ | Five specialized processes instead of one monolith: Channel (message routing), Branch (context assembly), Worker (tool execution), Compactor (memory), Corrector (error recovery). Proper Unix process isolation. | Praxis is single-process. A multi-process architecture would improve fault isolation and enable parallel phases. |

### Tier 2 — Medium Impact, Practical Features

| # | Feature | Source | Stars | What It Is | Why for Praxis |
|---|---------|--------|-------|-----------|----------------|
| 8 | **Proactive Agent Mode** | memU | 14k★ | Agent doesn't just react to messages — it proactively does things. Wakes up on schedule, checks conditions, initiates actions without being asked. | Praxis has cron but it's reactive. MemU's proactive model is: "I noticed X, so I did Y without being asked." Different from scheduled tasks. |
| 9 | **Code-First Agent API with 30+ Integrations** | OpenMolt | 34★ | Build agents programmatically with a code-first API. 30+ built-in integrations (Gmail, Slack, GitHub, Notion, Stripe, etc.). Zod-typed outputs for type safety. Scope-gated tool access per integration. | Praxis has Spotify and Google Meet integrations. A code-first API for adding integrations (via type-safe tool definitions) would make extension faster. |
| 10 | **Skill Packs** | Golem | 187★ | Bundled collections of skills you install as a pack (e.g., "developer pack", "finance pack"). Skills are organized and distributed in groups. | Praxis has individual skills. Skill packs would simplify onboarding and use-case targeting. |
| 11 | **Heartbeat Check-Ins** | LettaBot | 325★ | Regular heartbeat health checks across channels. Agent persists state and sends health signals even when not actively processing. | Praxis has heartbeat file + check script. LettaBot's model is more integrated — the agent actively checks in. |
| 12 | **Chinese Platform Channels** | OpenMozi, LangBot, Lilium AI | 180★ / 16k★ / 94★ | QQ, Feishu, DingTalk, WeChat, WeCom channels. Chinese LLM providers (DeepSeek, Doubao, Qwen, Kimi, Zhipu). | If Jeremy wants global reach, these are the platforms for the Chinese market. |
| 13 | **Mobile-Native Agent** | Kai | 720★ | Full OpenClaw alternative that runs natively on Android/iOS. Jetpack Compose UI, any OpenAI-compatible endpoint. First mobile-native claw. | Praxis could have a mobile app companion. Not necessarily the full agent, but a control surface / notification hub. |
| 14 | **Signed WASM Plugins** | Carapace | 43★ | WASM plugins with cryptographic signatures. Plugin code is verified before execution. Source-level audit trail. | Beyond IronClaw's WASM sandbox — adds code signing. Could prevent supply chain attacks on the skills/plugins marketplace. |
| 15 | **Scalable Embeddings Provider** | ClawSync | 69★ | Uses Together AI for embeddings and inference. Built on Convex (reactive backend). Multi-agent with shared soul documents and agent-to-agent communication. | Interesting multi-agent model with shared identity documents. Shared soul docs concept. |
| 16 | **Trigger-Based Tool Execution** | OpenLegion | 91★ | Tools can be triggered by events, not just LLM requests. Webhook → tool chain without LLM invocation for known patterns. | Efficient for well-defined workflows. Skip the LLM cost for deterministic tasks. |
| 17 | **16/32+ Built-in Tools** | ZeptoClaw | 619★ | Ships with 32 built-in tools and 9 channels. Claims "keeps the integrations, drops the bloat." | Praxis has 4 built-in tools (file-read, git-query, shell-exec, web-fetch). More built-in tools would improve out-of-box utility. |
| 18 | **Pi-Inspired Self-Extension (creates own skills at runtime)** | Moltis | 2.7k★ | Agent creates its own skills on-the-fly. Session branching, hot-reload of newly created skills. | Praxis has self-evolution proposals but not runtime skill creation. Moltis's agent can say "I need a new capability" and build it mid-session. |

### Tier 3 — Niche but Interesting

| # | Feature | Source | Notes |
|---|---------|--------|-------|
| 19 | **Whisper.cpp local STT** | OpenCrabs | Uses whisper.cpp for fully local speech-to-text. No cloud dependency. |
| 20 | **NVIDIA OpenShell sandbox** | NemoClaw | Policy-based privacy guardrails, inference routed through NVIDIA hardware. Enterprise-level GPU isolation. |
| 21 | **Cowork/UI overlay for coding agents** | AionUi | 24/7 coding agent UI that works with Claude Code, Codex, OpenCode, Gemini CLI — a unified interface over multiple agent backends. |
| 22 | **Embedded/Bare-metal target** | MimicLaw, ZClaw | Pure C on ESP32, no OS. 888 KiB firmware. GPIO control. |
| 23 | **Apple Container isolation** | NanoClaw | Per-channel lightweight VM isolation on macOS. Different from Docker. |
| 24 | **WebVM (v86) sandbox** | OpenBrowserClaw | x86 emulator in browser running Alpine Linux. Browser-based sandbox. |
| 25 | **TOML-based skill packs** | Golem | Skills as bundled, versioned `.toml` collections. |
| 26 | **Zod-typed tool outputs** | OpenMolt | TypeScript-first type safety for tool I/O. Could translate to Rust with serde + schemars. |
| 27 | **Onchain reputation system** | CashClaw | Agent has verifiable onchain reputation from completed work. Trust without central authority. |
| 28 | **Self-improvement from user ratings** | CashClaw, TinyClaw | Agent collects star ratings, adjusts behavior. Explicit feedback loop. |

---

## Projects Worth a Quick Look (Sorted by Stars)

| Project | Stars | Lang | Niche Feature |
|---------|-------|------|--------------|
| **Hermes Agent** | 136k★ | Python | Self-improving learning loop (Praxis has similar but different approach) |
| **Nanobot** | 42k★ | Python | MCP-first architecture — reference for thin-orchestrator design |
| **AstrBot** | 31k★ | Python | Multi-platform IM with Chinese platform support |
| **ZeroClaw** | 31k★ | Rust | 3-mode system (CLI/Gateway/Daemon), trait-driven providers |
| **NanoClaw** | 29k★ | TS | OS-level container isolation, 500 lines, agent swarms |
| **OpenFang** | 17k★ | Rust | "Agent OS" — 137K LOC, 1,767+ tests, 32MB binary |
| **memU** | 14k★ | Python | Proactive agent (not reactive) |
| **NemoClaw** | 20k★ | C | NVIDIA's OpenShell sandbox + policy guardrails |
| **MemOS** | 8.9k★ | Python | Persistent skill memory for cross-task reuse |
| **NullClaw** | 7.4k★ | Zig | 678KB binary, 22+ providers, 17 channels |
| **Spacebot** | 2.2k★ | Rust | Multi-process architecture (5 specialized daemons) |
| **CashClaw** | 994★ | TS | Onchain agent marketplace — agent gets paid for work |
| **OpenCrabs** | 708★ | Rust | whisper.cpp local STT, zeroized keys |
| **ZeptoClaw** | 619★ | Rust | 32 built-in tools, container sandboxing |
| **OpenBrowserClaw** | 592★ | TS | Browser-only PWA agent, zero infra |
| **Gitclaw** | 346★ | TS | Git-native agent lifecycle |
| **LettaBot** | 325★ | TS | Heartbeat check-ins across 5 channels |
| **TinyClaw** | 243★ | TS | Heartware personality system |
| **Golem** | 187★ | Go | Skill packs concept |
| **OpenLegion** | 91★ | Python | Docker-per-agent, event-driven tools |
| **Carapace** | 43★ | Rust | Signed WASM plugins |
| **OpenMolt** | 34★ | TS | Code-first API, 30+ integrations |

---

## Summary: Top 10 Features Praxis Should Consider

1. **Agent-as-Worker Marketplace** (CashClaw) — Aligns with Jeremy's "AI-powered service business" vision. Agent finds, prices, executes, and gets paid for work.
2. **Git-Native Agent Lifecycle** (Gitclaw) — Identity/rules/memory as version-controlled files. Fork, branch, diff agents.
3. **Zero-LLM Mode** (SafeClaw) — Rule-based deterministic behavior. Save API costs for routine tasks.
4. **Browser-Only PWA Mode** (OpenBrowserClaw) — Zero-infrastructure entry point for Praxis.
5. **Personality/Relationship System** (TinyClaw, Clawra) — Heartware: explicit personality model, moods, relationship memory.
6. **Multi-Process Architecture** (Spacebot) — Fault isolation via specialized processes (Channel, Worker, Compactor, Corrector).
7. **Proactive Agent Mode** (memU) — Agent initiates actions unprompted based on observed conditions.
8. **Code-First Integration API** (OpenMolt) — Type-safe, scope-gated tool definitions with 30+ pre-built integrations.
9. **Signed WASM Plugins** (Carapace) — Cryptographic verification on the skills/plugins marketplace.
10. **27+ More Built-in Tools** (ZeptoClaw) — Praxis has 4. ZeptoClaw ships 32 out of the box.
