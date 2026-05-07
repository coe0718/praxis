# Ecosystem Research — OpenClaw Alternatives & Praxis Feature Gaps

**Date:** 2026-05-06  
**Author:** Scout  
**Context:** Comprehensive survey of the Claw ecosystem (all OpenClaw alternatives) to identify features Praxis doesn't yet have.

---

## The Full Claw Ecosystem (2026)

The space exploded after OpenClaw's January 2026 launch by Peter Steinberger (creator of PSPDFKit). Within weeks, 12+ alternatives appeared. Here's everyone:

| Project | Language | Est. LoC | Niche |
|---------|----------|----------|-------|
| **Moltis** | Rust | ~270K | Persistent agent server — voice I/O, security, web UI |
| **IronClaw** | Rust | ~150K | Enterprise security — WASM sandbox, PostgreSQL |
| **ZeroClaw** | Rust | ~50K | Performance king — 3.4MB binary, <10ms boot |
| **MicroClaw** | Rust | ~30K | 14+ platform adapters |
| **NullClaw** | Zig | ~5K | 678KB binary, ~1MB RAM |
| **NanoClaw** | TypeScript | ~500 lines | Radical minimalism + OS-level container isolation |
| **OpenClaw** | TypeScript | ~400K+ | The original — 280K+ GitHub stars, ClawHub marketplace |
| **PicoClaw** | Go | ~8K | Runs on $10 hardware |
| **Nanobot** | Python | ~4K | MCP-first architecture (HKU academic project) |
| **Hermes Agent** | Python | ~152K | Self-improving learning loop (Nous Research) |
| **Claude Code Channels** | Python | N/A | Dev workflow agent (Anthropic) |
| **OpenCode** | Go | ~150K | Terminal coding agent |

### Where Praxis fits

Praxis is **the most complete Rust agent runtime** in the ecosystem — 59/61 gaps closed, covering agent loop, plugins, skills, tool approval, memory, scoring, self-evolution, MCP, kanban, curator, A2A sync, and more (see `STATUS.md`). The remaining 2 open gaps are external blockers (Vercel Sandbox, Discord Voice via Opus).

This document catalogs what the ecosystem has that Praxis doesn't.

---

## Features Found in Alternatives — NOT in Praxis

### 🟢 Easy Wins (small scope, high value)

| # | Feature | Source | What It Does | Est. Scope |
|---|---------|--------|-------------|-----------|
| 1 | **Hot-reload config** | IronClaw | Change `praxis.toml` without restarting the daemon | ~2 days |
| 2 | **Self-update** | Moltis, OpenClaw | `praxis update` — downloads and swaps binary | ~3 days |
| 3 | **Backup archives** | IronClaw | `praxis backup` — creates/verifies portable snapshots | ~2 days |
| 4 | **Embedding cache** | Moltis | Cache vector embeddings locally to reduce API calls | ~1 day |
| 5 | **Edit checkpoints** | Moltis | Auto-snapshot files before agent modifies them | ~3 days |

### 🟡 Medium Impact

| # | Feature | Source | What It Does | Est. Scope |
|---|---------|--------|-------------|-----------|
| 6 | **Signal channel** | Moltis, IronClaw | Encrypted messaging integration | ~1 week |
| 7 | **Matrix channel** | Moltis | Decentralized open-protocol messaging | ~1 week |
| 8 | **MS Teams channel** | Moltis, OpenClaw | Enterprise chat integration | ~1 week |
| 9 | **OpenTelemetry tracing** | Moltis | Distributed tracing (Praxis has Prometheus only) | ~1 week |
| 10 | **OpenAI Batch API** | Moltis | 50% cost reduction for async batch LLM jobs | ~3 days |
| 11 | **Push notifications (mobile)** | Moltis | Mobile push for approvals, alerts, briefings | ~1 week |
| 12 | **Passkeys (WebAuthn)** | Moltis | Passwordless authentication for dashboard | ~1 week |
| 13 | **Reciprocal Rank Fusion search** | IronClaw | Better hybrid vector+FTS ranking algorithm | ~3 days |
| 14 | **Multi-device sync** | Moltis | Sync session state/memory across machines | ~2 weeks |
| 15 | **WhatsApp channel** | OpenClaw, NanoClaw | WhatsApp messaging via Baileys library | ~1 week |
| 16 | **Gmail channel** | NanoClaw | Direct email integration via Gmail API | ~1 week |

### 🔴 High Impact (significant new capability)

| # | Feature | Source | What It Does | Est. Scope |
|---|---------|--------|-------------|-----------|
| 17 | **Voice I/O (TTS + STT)** | Moltis | 8 TTS + 7 STT providers. Praxis has "Whisper integration" listed as not-finished (STT-only). Adding full voice pipeline with provider pattern. | ~2 weeks |
| 18 | **WASM sandbox for tools** | IronClaw | Every tool runs in isolated WebAssembly container with capability-based permissions (filesystem, network, resources). Praxis uses filesystem sandboxing (`sandbox.rs`) — this is a fundamentally stronger isolation model. IronClaw's signature innovation. | ~1 month |
| 19 | **Full Web UI** | Moltis, IronClaw | Rich web dashboard with live settings, tools inventory, deploy key management, host pinning. Praxis dashboard is lightweight (SSE + metrics + TUI + SPA sidebar). A proper setup/management UI would be a massive usability win. | ~3-4 weeks |
| 20 | **Browser automation (containerized)** | Moltis | Run browser sessions in isolated Docker containers for safe web automation tasks | ~2 weeks |
| 21 | **SSH/remote execution** | Moltis | Execute tools on remote machines via SSH with key management | ~1 week |
| 22 | **Prompt injection detection** | IronClaw | Pattern detection + content sanitization on user inputs before they reach the LLM | ~1 week |
| 23 | **Credential leak detection** | IronClaw | Scan tool outputs for leaked API keys, tokens, secrets before returning to user | ~1 week |
| 24 | **OpenAI-compatible API endpoint** | IronClaw | Expose Praxis as `/v1/chat/completions` — other tools/agents can use Praxis as an LLM backend with model routing, fallback, and approval | ~1 week |
| 25 | **Sub-agent delegation (wire it up)** | Moltis | `src/delegation.rs` store exists but Act phase never sends work over links. Moltis has `spawn_agent` fully wired with nesting depth limits and tool filtering. **Lowest effort highest leverage** — store is ready, just needs Act integration. | ~3 days |
| 26 | **Agent swarms** | NanoClaw | Teams of collaborating agents working in parallel on decomposed tasks | ~2 weeks |
| 27 | **Routines engine with self-repair** | IronClaw | Beyond basic cron — event-driven triggers, webhook-reactive jobs, self-healing background workers with heartbeat monitors | ~2 weeks |
| 28 | **Dynamic tool building from NL** | IronClaw | "Create a tool that fetches weather data" → generates WASM tool from natural language description | ~2 weeks |
| 29 | **Cursor-compatible project context** | Moltis | Export project context in Cursor IDE-compatible format | ~2 days |
| 30 | **Context-file threat scanning** | Moltis | Scan uploaded files for malware/malicious content before processing | ~1 week |

---

## Key Competitor Deep Dives

### Moltis — The Most Complete Rust Competitor

**Repository:** [github.com/moltis-org/moltis](https://github.com/moltis-org/moltis)  
**Creator:** Fabien Penso  
**Language:** Rust (~270K LoC across 59 crates)  
**License:** MIT  
**Latest:** HN front page, active development

**What Moltis does that Praxis doesn't:**

- Voice I/O (8 TTS providers + 7 STT providers) — complete voice pipeline
- Full web UI with live Settings → Tools inventory, deploy key management, host pinning
- Channels: Signal, Matrix, Nostr, MS Teams (in addition to Telegram/Discord/Slack)
- Browser automation in isolated Docker containers
- SSH/node-backed remote execution
- Automatic edit checkpoints during file modifications
- Context-file threat scanning before processing
- Cursor-compatible project context export
- OpenTelemetry tracing (Praxis has Prometheus only)
- OpenAI Batch API integration (50% cost reduction)
- Pi-inspired self-extension (agent creates its own skills at runtime)
- Mobile PWA with push notifications
- Multi-device sync
- Passkeys (WebAuthn) authentication
- Embedding cache

**Architecture:** Gateway (Axum HTTP/WS) → Chat Service → Agent Runner + Tool Registry → Provider Registry. Modular 59-crate workspace.

**Takeaway:** Moltis is Praxis's closest Rust competitor. They're building the same thing but Moltis has a richer web UI and voice pipeline. Praxis has a stronger agent loop (Orient/Decide/Act/Reflect), self-evolution, kanban, curator, scoring, and LanceDB memory. Different architectural philosophies in the same space.

---

### IronClaw — Security-First Rust Agent OS

**Repository:** [github.com/nearai/ironclaw](https://github.com/nearai/ironclaw)  
**Language:** Rust (~150K LoC)  
**License:** BSL 1.1  
**Niche:** Enterprise security, zero-trust sandboxing

**What IronClaw does that Praxis doesn't:**

- **WASM sandbox for every tool** — isolated WebAssembly containers with capability-based permissions (filesystem, network, process). This is their killer feature. Tools run: WASM → allowlist → leak scan → credential injector → execute → leak scan → WASM output.
- PostgreSQL + pgvector for hybrid vector + full-text search with Reciprocal Rank Fusion
- Formal verification of critical security paths
- Prompt injection pattern detection + content sanitization
- Credential leak detection (scans tool outputs for leaked secrets)
- Endpoint allowlisting for SSRF protection
- Routines engine with self-repair heartbeat (beyond basic cron)
- Parallel isolated job execution with self-repair
- Dynamic WASM tool building from natural language descriptions
- OpenAI-compatible API endpoints (`/v1/chat/completions`, `/v1/models`)
- Configuration hot-reload
- Tailscale integration for mesh networking
- Bonjour/mDNS discovery for LAN
- APNs push notification pipeline
- Trusted-proxy auth mode
- Backup CLI commands

**Architecture:** Agent Loop → Router (intent classification) → Scheduler (parallel jobs with priorities) → Worker → Orchestrator (container lifecycle, LLM proxying). 5-layer security model.

**Takeaway:** IronClaw is the security-focused alternative. Their WASM sandbox is the single most differentiating feature in the entire ecosystem. The credential leak detection and prompt injection defense are practical security wins. PostgreSQL+pgvector is heavier than SQLite+LanceDB but enables more sophisticated queries.

---

### Nanobot — MCP-First Minimalism

**Repository:** [github.com/HKUDS/nanobot](https://github.com/HKUDS/nanobot)  
**Language:** Python (~4,000 lines)  
**License:** MIT  
**Stars:** ~26,800  
**Niche:** Ultra-lightweight MCP-centric architecture

**What Nanobot does that Praxis doesn't:**

- **MCP-first architecture** — Acts as a thin orchestrator; all functionality lives in external MCP tool servers. Adding capabilities = plugging in new MCP servers, not modifying core code.
- Embeddable as a library in other Python projects
- Clean ~4K line codebase that's fully auditable

**Architecture:** MessageBus (asyncio pub-sub) → AgentLoop (20-iteration cap) → ContextBuilder (assembles from SOUL.md, USER.md, memory, skills) → SkillsLoader → MemoryStore.

**Takeaway:** Nanobot isn't a direct competitor (Python, not Rust) but its MCP-first architecture is an interesting design pattern. Praxis already has MCP tool discovery wired at startup, so it's architecturally close.

---

### ZeroClaw — Performance King

**Repository:** [github.com/zeroclaw-labs/zeroclaw](https://github.com/zeroclaw-labs/zeroclaw)  
**Language:** Rust (~50K LoC)  
**License:** MIT  
**Stars:** ~26,200  
**Niche:** Minimum footprint, edge deployment

| Metric | OpenClaw | ZeroClaw |
|--------|----------|----------|
| Binary size | ~200MB+ | **3.4MB** |
| Cold boot time | Several seconds | **<10ms** on 0.6GHz cores |
| RAM usage | Hundreds of MB | **<5MB** |
| Min hardware | Mac mini (~$500+) | **$10 hardware** |

**What ZeroClaw does that Praxis doesn't:**

- Trait-driven architecture with easy LLM/channel swapping
- 22+ LLM providers out of the box
- Three modes: Agent (CLI), Gateway (HTTP), Daemon
- Sub-10ms boot + <5MB RAM targets

**Takeaway:** ZeroClaw is optimized for a niche Praxis doesn't target (extreme low-resource edge). Not directly competitive but the trait-driven provider pattern is a nice design reference.

---

## Recommended Priority Order for Praxis

If picking features to add, here's the tiered recommendation:

### Tier 1 — Lowest Effort, Highest Leverage

1. **Wire up sub-agent delegation** (`src/delegation.rs`) — Store exists, Act phase just needs to use it. ~3 days.
2. **OpenAI-compatible API endpoint** — Expose Praxis as a service other agents can call. ~1 week.
3. **Hot-reload config** — Stop-start cycle is friction. ~2 days.

### Tier 2 — Differentiating Capabilities

4. **Voice I/O (TTS + STT)** — Whisper is "not finished yet." Add provider pattern for TTS. ~2 weeks.
5. **Browser automation** — Containerized browser for agent-driven web tasks. ~2 weeks.
6. **Prompt injection + credential leak detection** — Makes Praxis "the safe agent." ~2 weeks combined.

### Tier 3 — The Hard One (Most Differentiating)

7. **WASM sandbox for tools** — IronClaw's killer feature. Hardest but most differentiating. Would make Praxis the only Rust agent with WASM-isolated tools. ~1 month.

### Tier 4 — Channel Expansion

8. **Signal channel** — Growing demand for encrypted messaging.
9. **Matrix channel** — Decentralized protocol.
10. **WhatsApp channel** — Largest messaging platform.

### Tier 5 — Polish & Ecosystem

11. Self-update + backup
12. OTel tracing
13. Embedding cache
14. Edit checkpoints
15. Push notifications
16. Multi-device sync

---

## Sources

- [OpenClaw Alternatives Comparison 2026 — AI Magicx](https://www.aimagicx.com/blog/openclaw-alternatives-comparison-2026)
- [The Six Claws — ibl.ai](https://ibl.ai/blog/the-six-claws-a-field-guide-to-open-source-ai-agent-frameworks)
- [Moltis GitHub](https://github.com/moltis-org/moltis)
- [Moltis Homepage](https://moltis.org/)
- [IronClaw GitHub](https://github.com/nearai/ironclaw)
- [IronClaw Feature Parity Matrix](https://github.com/nearai/ironclaw/blob/main/FEATURE_PARITY.md)
- [Best Self-Hosted AI Agents 2026 — Lushbinary](https://lushbinary.com/blog/best-self-hosted-ai-agents-hermes-openclaw-ironclaw-compared/)
- [Nanobot GitHub](https://github.com/HKUDS/nanobot)
- [Rust Agent Runtime Showdown: MicroClaw vs ZeroClaw vs Moltis](https://medium.com/@everettjf/rust-agent-runtime-showdown-microclaw-vs-zeroclaw-vs-moltis-df1ecb85c676)
- [The Claw ecosystem: 12 personal agents, dissected](https://michaellivs.com/blog/personal-ai-agents-compared/)
