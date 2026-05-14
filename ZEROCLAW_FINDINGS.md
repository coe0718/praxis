# ZeroClaw Analysis — Features of Interest for Praxis

**Date:** 2026-05-14
**Source:** https://github.com/zeroclaw-labs/zeroclaw (346K lines Rust)
**License:** MIT / Apache 2.0
**Version:** 0.7.5
**Crates:** 17 workspace crates + apps, firmware, Tauri desktop, web dashboard
**Rust edition:** 2024 (requires Rust 1.87)

---

## Overview

ZeroClaw is the largest, most mature Rust agent runtime in the open-source ecosystem (31K ★). A single Rust binary (~6.6 MB minimal, ~30 MB full) with 30+ channels, 22+ LLM providers, hardware GPIO support, firmware targets, and a full security subsystem. The closest direct Rust competitor to Praxis.

---

## High-Value Features Praxis Should Consider Absorbing

### 1. SOP Engine — Declarative Workflow Engine (CRITICAL)

`crates/zeroclaw-runtime/src/sop/` (3,900+ lines total) — Standard Operating Procedures engine:

- SOPs are TOML + Markdown files defining multi-step workflows
- **Triggers:** event-based, keyword, cron, file-watch, and webhook triggers
- **Execution modes:** auto (fully autonomous), semi (human approval gates), manual (human-in-the-loop)
- **Step types:** llm_call, tool_exec, code_exec, http_request, wait_for_approval, sub_sop, condition, parallel, loop
- **Deterministic mode:** Cache LLM responses for identical inputs (saves tokens, guarantees repeatability)
- **Priority system:** Critical > High > Normal > Low with preemption
- **Cooldowns, max concurrent, conditions on transitions**
- **Metrics tracking:** Token savings, run duration, step-level timing
- **Dispatch:** Routes SOP events to a work queue for processing

**Why for Praxis:** This is a full workflow engine baked into the agent runtime. No other open-source agent has deterministic SOP execution with human approval gates. Praxis's kanban system is complementary (task management), but SOP is procedural workflow.

---

### 2. Hands — Declarative Recurring Scheduled Agents (HIGH)

`src/hands/` — TOML-defined recurring agent tasks with stateful context:

```toml
name = "scanner"
description = "Market scanner"
prompt = "Scan markets for interesting activity."

[schedule]
kind = "cron"
expr = "0 9 * * *"
```

- Each "hand" gets its own rolling context file (`{name}/context.json`)
- Accumulates `findings` and `learned_facts` across runs (capped history)
- `HandRun` tracks: hand_name, run_id, started_at, finished_at, status, findings, knowledge_added, duration_ms
- Auto-caps history to `max_history` entries
- Persists knowledge across restarts

**Why for Praxis:** Simpler than a full SOP but more powerful than a bare cron job. Each hand builds institutional knowledge across runs — like a recurring analyst that remembers what it learned last time.

---

### 3. Knowledge Graph Memory (HIGH)

`crates/zeroclaw-memory/src/knowledge_graph.rs` (863 lines) — SQLite-backed knowledge graph:

- **5 node types:** Pattern, Decision, Lesson, Expert, Technology
- **5 directed relations:** Uses, Replaces, Extends, AuthoredBy, AppliesTo
- Full-text search across node content, tags, and titles
- Bidirectional relation traversal
- Node metadata: title, content, tags, source, confidence, timestamps
- Separate PostgreSQL-backed knowledge graph for production deployments

**Why for Praxis:** This is a structured alternative to markdown file memory. A knowledge graph captures relationships between facts (e.g., "Rust's `tokio` uses async I/O, replaces thread-per-connection"). Much richer than flat MEMORY.md. Perfect for capturing expertise over time.

---

### 4. Approval System with Autonomy Levels (HIGH)

`crates/zeroclaw-runtime/src/approval/` (810 lines) — Per-tool approval gateway:

- **Autonomy levels:** `ReadOnly` (block all mutations), `Supervised` (ask for approval on medium-risk), `Full` (no prompts)
- **Configurable per-tool:** `auto_approve` list (skip prompt), `always_ask` list (always prompt)
- **Session allowlist:** "Always approve for this session" — persists for the session's duration
- **Non-interactive mode:** Channels always run in this mode — unknown tools get auto-denied
- **Full audit log:** Every decision recorded with timestamp, channel, tool_name, arguments summary, response
- **Argument summarization:** Sensible truncation of long args for approval prompts
- `always_ask` overrides session allowlist (safety guarantee)

**Why for Praxis:** Praxis has no user-facing approval system for tool execution. This is essential for safe multi-channel operation — you don't want Telegram users running `shell` commands without approval.

---

### 5. Verifiable Intent / Trust System (MEDIUM-HIGH)

`crates/zeroclaw-runtime/src/verifiable_intent/` — Full SD-JWT layered credential system:

- L2/L3 credential issuance and chain verification
- Constraint-based policy evaluation for commerce-gated actions
- Three-layer credential chain: L1 (root of trust) → L2 (authority) → L3 (operation)
- KB-SD-JWT (Key-Bound Selective Disclosure JWT) for privacy-preserving verification
- Machine-readable error taxonomy

`crates/zeroclaw-runtime/src/trust/` — Trust scoring and policy:
- Trust levels for tool access
- Reputation tracking across sessions

**Why for Praxis:** Bleeding-edge security model. Verifiable Intent is an emerging spec for agent-to-agent commerce. Having this positions Praxis for agent economy use cases. Low urgency but strategically valuable.

---

### 6. Channel Orchestrator (MEDIUM)

`crates/zeroclaw-channels/src/orchestrator/` (13,453 lines) — Central channel lifecycle manager:

- Message routing across 30+ channel types
- Media pipeline (image resize, format conversion, caching)
- Session management per channel
- Stall watchdog for hung calls
- Debounce layer for noisy channels
- Link enrichment (expand URLs to rich previews)
- Transcription pipeline for voice messages
- TTS for text-to-speech responses
- Channel health monitoring

Previously noted channels: Discord, Telegram, Matrix, Signal, WhatsApp, Whatsapp Web, WeChat, QQ, Lark, DingTalk, Bluesky, Twitter, Reddit, Notion, Nextcloud Talk, iMessage, IRC, Mattermost, Email, Gmail Push, Slack, Webhook, ACP Server, Voice Call, Voice Wake, Linq, WATI, Mochat, WeCom, ClawdTalk.

**Why for Praxis:** This is the channel ecosystem goal — but at 13K lines it's a massive undertaking to replicate. Worth studying the architecture patterns (routing, media pipeline, session backend traits) rather than porting directly.

---

### 7. Memory Storage Ecosystem (MEDIUM)

`crates/zeroclaw-memory/` (11,913 lines across 20+ files) — Comprehensive memory backends:

| Backend | Lines | Purpose |
|---------|-------|---------|
| `sqlite.rs` | 2,948 | Primary persistent store |
| `knowledge_graph.rs` | 863 | Structured knowledge graph |
| `lucid.rs` | 726 | Dream-style consolidation |
| `qdrant.rs` | 669 | Qdrant vector DB (dense retrieval) |
| `hygiene.rs` | 586 | Data lifecycle management (TTL, compaction) |
| `response_cache.rs` | 526 | LLM response caching (deterministic mode) |
| `postgres.rs` | 509 | PostgreSQL backend |
| `snapshot.rs` | 470 | Point-in-time memory snapshots |
| `markdown.rs` | 430 | Markdown file-based memory |
| `vector.rs` | 403 | Generic vector DB abstraction |
| `chunker.rs` | 377 | Document chunking for RAG |
| `embeddings.rs` | 358 | Embedding generation abstraction |
| `retrieval.rs` | 266 | Retrieval API |
| `consolidation.rs` | 239 | Memory consolidation engine |

**Why for Praxis:** The memory crate is more mature than Praxis. Key ideas: snapshot-based recovery, chunker+retrieval for RAG, response_cache for deterministic execution, and the memory consolidation engine.

---

### 8. WebAuthn / Hardware Key Authentication (MEDIUM)

`crates/zeroclaw-runtime/src/security/webauthn.rs` (1,374 lines) — Server-side WebAuthn:

- Registration (attestation) and authentication (assertion) flows
- FIDO2/WebAuthn hardware key support (YubiKey, SoloKey, platform authenticators)
- Credential serialization via SecretStore (AEAD encrypted)
- SQLite-backed credential database
- Built using `ring` for crypto (no heavy third-party libs)
- Challenge/response protocol with ES256 (ECDSA P-256)

**Why for Praxis:** Gateway/dashboard authentication without passwords. Hardware key auth is increasingly expected for admin interfaces.

---

### 9. Swarm Multi-Agent Orchestration (MEDIUM)

`crates/zeroclaw-tools/src/swarm.rs` (967 lines) — Tool-based swarm orchestration:

- **3 strategies:** Sequential (pipeline), Parallel (fan-out/fan-in), Router (LLM chooses agent)
- Configurable agents with per-agent provider, model, API key
- Configurable swarms with strategy selection
- 120s default timeout with configurable per-agent
- Built as a Tool, so agents can spawn swarms mid-conversation

**Why for Praxis:** Different from Praxis's subagent approach. The Swarm tool is agent-callable (the agent decides to use it), and supports pipelines of agents. Especially useful for multi-step research tasks.

---

### 10. Identity / AIEOS System (MEDIUM)

`crates/zeroclaw-runtime/src/identity.rs` (1,488 lines) — AIEOS v1.1 identity specification:

- Portable AI identity in JSON format: names, bio, origin, psychology (MBTI/OCEAN), linguistics (formality, catchphrases), motivations, capabilities, physicality, history, interests
- Converts AIEOS identity to system prompt format
- Dual format support: OpenClaw (markdown) and AIEOS (JSON)

**Why for Praxis:** Standardized agent identity that can be shared/ported between runtimes. If multi-agent or agent-to-agent interaction is on the roadmap, having a standard identity format is valuable.

---

### 11. Security Infrastructure (MEDIUM)

`crates/zeroclaw-runtime/src/security/` multiple files:

- **SecretStore:** AEAD (ChaCha20Poly1305) encrypted credentials on disk
- **SecurityPolicy:** Autonomy levels, sandbox config, workspace boundaries
- **PairingGuard:** Device pairing for channel authorization
- **Audit Log:** Tamper-evident tool execution log
- **Redact:** API key redaction in log output (char-boundary-safe for UTF-8)
- **Sandbox backends:** Landlock (kernel-level), Bubblewrap (user ns), Seatbelt (macOS), Docker
- **Hooks:** Pre/post execution hooks for security scanning

**Why for Praxis:** The Policy + Autonomy + Sandbox + Audit combo is a complete security model. The sandbox abstraction with multiple OS backends is production-grade.

---

### 12. Tunnel Providers (LOW-MEDIUM)

`src/tunnel/` — Factory-pattern tunnel providers:

| Provider | Use case |
|----------|----------|
| Cloudflare Tunnel | Production exposure |
| Tailscale | Private network |
| ngrok | Development |
| Pinggy | Lightweight |
| OpenVPN | VPN-based |

Factory pattern: `create_tunnel(&config) -> Option<Box<dyn Tunnel>>`

**Why for Praxis:** Useful for exposing the Praxis gateway from a home server without static IP. The factory pattern makes adding providers trivial.

---

### 13. Firmware / Hardware Support (LOW for now)

`firmware/` — 9 firmware targets across 4 MCU families:

| Target | Platform |
|--------|----------|
| Arduino | Uno Q Bridge |
| ESP32 | Native + UI (TFT display) |
| Nucleo | STM32 for robotics |
| Raspberry Pi Pico | Embedded |

ZeroClaw communicates with these over serial/I2C/SPI via the `Peripheral` trait (extensible via `zeroclaw-api/src/peripherals_traits.rs`).

`crates/robot-kit/` (3,468 lines) — Robot control library for motor drivers, sensors, servos.

**Why for Praxis:** Niche but strategically differentiating. No other agent runtime has embedded hardware support. If Praxis targets IoT or robotics, these patterns are reusable.

---

## Other Notable Features

- **22+ LLM providers** with `ReliableProvider` wrapper (retry, fallback) and provider routing
- **Observability:** Prometheus metrics + OpenTelemetry tracing
- **Context compression:** Turn-based + LLM-summary (similar to AstrBot)
- **MCP client:** Full protocol with stdio/HTTP/SSE transports
- **Platform module:** OS-level integration (systemd, launchd, Windows Service)
- **Doctor CLI:** Runtime diagnostics, connectivity checks, config validation
- **Daemon mode:** Background service with health checks
- **Heartbeat:** Regular activity pings to detect agent stalls
- **Cron:** SQLite-backed scheduler with task persistence
- **Skills system:** User-defined skills with auditing and creator tool
- **Skillforge:** Skill creation/evolution tooling
- **Agent loop:** 8K lines of core agent loop with history pruning, context compression
- **Tools:** 80+ tool files (browser, shell, file edit/write, web fetch/search, git, Jira, LinkedIn, Notion, Google Workspace, weather, calculator, PDF read, LLM task, and more)
- **Web Dashboard:** React/TypeScript SPA with chat, memory browsing, config, cron management, tool inspection
- **Tauri desktop app:** Native desktop client
- **RAG:** Hardware-focused RAG for datasheets
- **ACP bridge:** Agent Communication Protocol bridge

---

## Architecture for Praxis Consideration

```
                       ┌─────────────────────┐
                       │     Front Door       │
                       │  CLI / Gateway / ACP │
                       └──────────┬──────────┘
                                  │
                       ┌──────────▼──────────┐
                       │    Agent Loop       │
                       │ (loop_.rs, 8K LOC)  │
                       └──────────┬──────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              │                   │                   │
     ┌────────▼────────┐ ┌───────▼───────┐ ┌─────────▼─────────┐
     │   Channels      │ │   Security   │ │      Memory       │
     │ 30+ adapters    │ │ Policy/Approval│ │ SQLite/KG/Vector  │
     │ Orchestrator    │ │ WebAuthn/Audit │ │ Consolidation     │
     └────────────────┘ └───────────────┘ └───────────────────┘
              │                   │                   │
     ┌────────▼────────┐ ┌───────▼───────┐ ┌─────────▼─────────┐
     │     Tools       │ │    SOP/Hands  │ │     Providers     │
     │ 80+ tool files  │ │ Workflow Eng. │ │ 22+ LLM providers │
     │ MCP client      │ │ Deterministic │ │ Fallback chains   │
     └────────────────┘ └───────────────┘ └───────────────────┘
```

Key architectural insight: Security (policy approval) wraps every tool call. Memory is multi-backend with a common trait. Channels all share an orchestrator for lifecycle management.

---

## What ZeroClaw Does NOT Have (Already in Praxis)

- Kanban / task decomposition
- Curator scoring system
- Self-evolution / reflection loop
- Agent warm-up / lifecycle hooks (different from ZeroClaw's hooks)
- Dream-style git-backed memory (ZeroClaw has `lucid` consolidation)
- Lightweight design (346K Rust vs Praxis's smaller footprint)

---

## How ZeroClaw Compares to Previously Analyzed Projects

| Feature | Nanobot | AstrBot | ZeroClaw | Praxis |
|---------|---------|---------|----------|--------|
| Language | Python | Python | Rust | Rust |
| Lines | 4K core | 112K | 346K | ~50K |
| Channels | 18 | 18 | 30+ | 1 (Discord) |
| Memory | Dream (git) | SQLite | SQLite+KG+Vector | Markdown |
| KB/RAG | None | Hybrid Dense+Sparse | Hardware RAG | None |
| Workflow | None | Pipeline (onion) | SOP Engine | Kanban |
| Security | Basic | Basic | Full (approval, WebAuthn) | None |
| MCP | Yes | Yes | Yes | No |
| Hardware | None | None | GPIO, firmware | None |
| TTS/STT | None | 12+ TTS providers | Voice call/wake | None |

---

## Recommendation

**Highest ROI for Praxis:**

1. **Approval System** (#4) — This is the most immediately applicable feature. A tool-level approval gateway with autonomy levels (ReadOnly/Supervised/Full) is essential before Praxis can safely add more channels. Without it, adding Telegram/Slack means anyone with access can run `shell`.

2. **Hands** (#2) — Declarative recurring agents with rolling context. Much simpler to implement than a full SOP engine. Each "hand" is just a TOML file + a cron schedule + context persistence. This overlaps with but extends Praxis's cron system.

3. **Knowledge Graph Memory** (#3) — Structured alternative to flat memory. The 5 node types + 5 relation types capture real expertise. Good complement to Dream-style git-backed memory from the Nanobot analysis.

4. **SOP Engine** (#1) — Long-term play. A workflow engine with deterministic execution is a genuine differentiator. But it's 4K lines of Rust — a multi-week port.

5. **Tunnel Providers** (#12) — Easy win. Factory pattern + 5 tunnel implementations. Makes self-hosted deployment from home networks trivial.

**Strategic note:** ZeroClaw is the closest Rust competitor to Praxis. It's 7x larger, has 30x more channels, and has a complete security model. But Praxis has kanban/curator/scoring/self-evolution that ZeroClaw lacks. The differentiation is clear: Praxis is a platform for building agent teams, ZeroClaw is a personal assistant. These findings highlight what Praxis should absorb from ZeroClaw's personal assistant features.

---

*Clone deleted.*
