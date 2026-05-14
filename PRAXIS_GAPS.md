# Praxis Gap Analysis — Features Actually Missing

**Generated:** 2026-05-14
**Source analysis of:** Nanobot, AstrBot, ZeroClaw, OpenClaw, PicoClaw, NanoClaw, NemoClaw, OpenFang, AionUi
**Praxis baseline:** ~67,500 lines Rust across ~80 modules

*This is the cleaned-up version — only features Praxis genuinely lacks, verified against the actual codebase.*

---

## Priority 1 (Highest ROI)

### 1. Dream — Git-Backed Memory with Line-Age Annotation
**Source:** Nanobot

Praxis has memory tiers (hot/cold) with LanceDB vector storage. What it doesn't have: Nanobot's Dream uses `git blame` to annotate every line in MEMORY.md with its age (`← 30d`), so the LLM can visually identify stale facts. Changes are auto-committed with full git history for revert. This is a **different approach** — instead of structured storage, it keeps human-readable markdown that the LLM can reason about with age context.

**Why it matters:** The line-age annotation solves a real problem — agents don't know which memories are stale. "I learned this 3 months ago" is invisible to the LLM in raw text.

### 2. Commitments — Proactive Agent Follow-Ups
**Source:** OpenClaw

The agent autonomously schedules follow-up reminders extracted from conversation. Four kinds: `event_check_in` (did X happen?), `deadline_check` (deadline approaching?), `care_check_in` (check on someone), `open_loop` (unresolved item). LLM-extracted with confidence scoring (threshold 0.72), SQLite persistence, heartbeat-driven delivery. Deduplication by key.

**Why it matters:** This is the proactive counterpart to Praxis's reflective memory. The agent doesn't just remember the past — it acts on future promises.

### 3. Crestodian — Structured Health Overview + Autonomous Repair
**Source:** OpenClaw

A separate CLI entrypoint that scans the entire installation and produces a structured health overview: config validity, channel connectivity, credential status, memory health, gateway status. An LLM assistant analyzes the overview and generates a repair plan with structured actions. The operator reviews and approves. This is **meta-operations** — not the same as curator/quality which scores agent performance.

**Why it matters:** When something breaks (Discord token expired, config corrupted), there's no way to diagnose and repair from within the agent loop. A separate health CLI breaks the circular dependency.

### 4. Hybrid Knowledge Base (Dense + Sparse + RRF Fusion)
**Source:** AstrBot

Praxis has embedding + memory. AstrBot's KB has full hybrid retrieval: FAISS vectors (dense) + BM25 (sparse) fused via Reciprocal Rank Fusion, with configurable reranker providers (NVIDIA, Xinference, vLLM). 6 document parsers (PDF, EPUB, URL, MarkItDown, text, plain). Configurable chunking (recursive, fixed-size). Session-level KB binding.

**Why it matters:** Dense + sparse retrieval with RRF outperforms pure vector search on mixed document types. The document parser ecosystem handles real-world file formats Praxis users will throw at it.

---

## Priority 2 (Medium-High)

### 5. Shields — Time-Bounded Break-Glass Lockdown
**Source:** NemoClaw (OpenShell)

Praxis has per-channel sandbox policies (tool allow/deny, security levels). It doesn't have a lockdown toggle that makes config immutable (chattr +i, 444 root:root) with automatic time-bounded restore. The sandbox **cannot lower its own shields** — only the host can. Detached timer process prevents permanent exposure if admin walks away. Full audit log.

**Why it matters:** Production deployment pattern. Operators lock down the agent and temporarily open a maintenance window with auto-close.

### 6. Agent Manifest System (Pre-Built Specialist Agents)
**Source:** OpenFang

OpenFang ships 32 pre-defined agents as TOML manifests with model config, capability declarations, resource limits (`max_llm_tokens_per_hour`), fallback models, and per-agent tool permissions. Praxis has tools and agents but no declarative manifest format for packaging and distributing agent personas.

**Why it matters:** A manifest format makes agents shareable and composable. The 32 specialists (orchestrator, coder, researcher, security-auditor, etc.) are immediately usable references.

### 7. Container-Based Per-Session Isolation
**Source:** NanoClaw

NanoClaw runs each session in its own Docker container with a dual-SQLite I/O surface: `inbound.db` (host writes → container reads) and `outbound.db` (container writes → host reads). No IPC, no shared memory, no stdin piping. Heartbeat via file touch. Contrasts with Praxis's in-process model.

**Why it matters:** True isolation for multi-tenant or untrusted workloads. Each session gets its own filesystem, MCP servers, temp space. Trade-off: higher resource usage vs. in-process.

---

## Priority 3 (Nice-to-Have)

### 8. Knowledge Graph Memory
**Source:** ZeroClaw

SQLite-backed graph with 5 node types (Pattern, Decision, Lesson, Expert, Technology) and 5 directed relations (Uses, Replaces, Extends, AuthoredBy, AppliesTo). Full-text search, tag filtering, bidirectional traversal. Separate PostgreSQL-backed version for production.

**Why it matters:** Captures *relationships* between facts, not just facts themselves. "Tokio uses async I/O and replaces thread-per-connection" is more valuable than two separate entries.

### 9. Seahorse Hierarchical LLM Summarization
**Source:** PicoClaw

Instead of Praxis's turn-based context compression, Seahorse builds a hierarchical summary tree: raw messages → short summaries → condensed summaries at depth. FTS5-indexed for search. Session-aware retrieval provides recent messages + relevant summaries within token budget. Async background compaction.

**Why it matters:** Hierarchical summarization preserves signal better than turn-dropping. Old conversations remain searchable via their summaries.

### 10. Security Audit System
**Source:** OpenClaw

81 files scanning every aspect for vulnerabilities: channel DM policies, filesystem permissions, gateway TLS, plugin code safety, credential hygiene. Each finding has `checkId`, `severity`, `detail`, `remediation`. Machine-readable and user-actionable. Different from quality — this is about operational security posture.

**Why it matters:** "Is my config safe?" is a question the operator should be able to ask and get a structured answer.

### 11. Hands with Rolling Context Across Runs
**Source:** ZeroClaw

ZeroClaw's Hands are TOML-defined recurring agents where each "hand" accumulates `findings` and `learned_facts` across runs in a rolling context file. More sophisticated than cron — each run builds on knowledge from previous runs. History is capped but knowledge persists.

**Why it matters:** A daily briefing agent that remembers what it noticed yesterday is more useful than one that starts fresh every run.

### 12. Tunnel Providers
**Source:** ZeroClaw

Factory-pattern tunnel providers: Cloudflare Tunnel, Tailscale, ngrok, Pinggy, OpenVPN. For exposing the gateway from a home server without static IP. Simple trait + factory pattern — easy to add providers.

**Why it matters:** Self-hosted agents behind NAT need tunnels. This is a well-defined interface with multiple implementations.

---

## What I Was Wrong About (Features Praxis Already Has)

| I recommended... | From... | But Praxis has... |
|-----------------|---------|------------------|
| WASM sandbox with capabilities | NemoClaw, IronClaw | `src/wasm/` — wasmtime + fuel + proxy + leak detect |
| Plugin system | NanoClaw, OpenClaw | `src/plugins/` — TOML manifests, hooks, tool/msging registration, marketplace |
| Plugin signing | OpenFang | `src/plugin_signing/` — Ed25519 |
| Credential vault | OpenFang, PicoClaw | `src/vault/` — TOML secrets with env refs |
| Evolution/self-improvement | PicoClaw | `src/evolution/` — two-step approval, JSONL log |
| TTS/STT providers | Moltis, AstrBot | `src/voice/` — configurable providers |
| MCP client | Nanobot | `src/mcp/` — 807 lines, full implementation |
| Multi-channel messaging | AstrBot, ZeroClaw | `src/messaging/` — 4,655 lines, multiple platforms |
| Embedding/vector search | AstrBot | `src/embedding/` + `src/memory/lance.rs` — LanceDB |
| OAuth flows | OpenFang | `src/oauth/` — 984 lines across 8 files |
| Agent-to-agent routing | NanoClaw | `src/a2a/` — 267 lines |
| Learning from experience | PicoClaw | `src/learning/` — 608 lines |
| Context assembly/compaction | OpenClaw | `src/context/` — 1,969 lines |
| Skills system | Nanobot | `src/skills/` — 661 lines |

---

**Bottom line:** Praxis is already more mature than most of the projects I analyzed. The genuine gaps are mostly about **new interaction models** (commitments, health overview, shields lockdown) and **data representation** (line-age annotation, knowledge graphs, hierarchical summarization) — not core infrastructure.
