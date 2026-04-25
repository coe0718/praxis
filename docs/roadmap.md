# Praxis Roadmap — Missing Features & Future Direction

This document captures everything identified as missing or incomplete during a
full project review (2025-04-23). Items are ranked by impact × effort.

## 🔴 Critical Gaps (High Impact, Foundation-Blocking)

### 1. MCP (Model Context Protocol) — `src/mcp/` is empty
The biggest architectural gap. MCP is becoming the industry standard for how
LLM agents expose and consume tools.

**What it unlocks:**
- Praxis tools become usable from Claude Desktop, Cursor, Windsurf, etc.
- Praxis can consume external MCP servers (filesystem, GitHub, databases, Brave
  Search, etc.) without custom integration code.
- Standardised inter-agent tool interoperability.

**Implementation notes:**
- Build an MCP *server* that exposes Praxis tools via stdio/SSE transport.
- Build an MCP *client* that connects to external MCP servers and surfaces their
tools in the Praxis tool registry.
- Follow the Anthropic MCP spec (JSON-RPC over stdio or HTTP+SSE).

---

### 2. Streaming LLM Responses
The entire backend uses `reqwest::blocking`. Every provider call blocks until
the full response arrives.

**Why it matters:**
- No progress indication in the dashboard during long `decide`/`act` phases.
- SSE events are fake (just completion notifications, not real streaming).
- Precludes real-time tool-use chains (Claude extended thinking, reasoning
  tokens).

**Implementation notes:**
- Migrate backend to async `reqwest` with `eventsource` or SSE parsing.
- Stream tokens into the dashboard via real SSE.
- Token counting becomes approximate until the final usage block arrives.

---

### 3. A2A (Agent-to-Agent) Protocol — `src/delegation.rs` has no wire format
Google's A2A spec is gaining traction. Praxis has delegation logic but no
standardised way for Praxis agents to talk to AutoGen, CrewAI, LangGraph, etc.

**What it unlocks:**
- Cross-platform agent swarms.
- External orchestrators can delegate to Praxis workers.
- Praxis can call specialist agents for specific tasks.

---

### 4. Structured Logging + Observability
`env_logger` is the only logging. Production deployments need:

| Need | Current State | Target |
|------|---------------|--------|
| JSON logs | Plain text | `tracing-subscriber` with JSON layer |
| Distributed traces | None | OpenTelemetry spans across provider calls |
| Metrics | None | Prometheus endpoint (`/metrics`) for token usage, latency, errors |
| Log aggregation | Manual | Structured output compatible with Datadog/ELK |

---

## 🟠 Important Features (Medium-High Impact)

### 5. Config Hot-Reload
The daemon (`praxis run`) loads `praxis.toml` once at startup. Operators must
restart to change:
- Backend provider
- Budget policy
- Context priorities
- Security level

**Implementation:** `notify` crate watching the config file, with validation
before applying.

---

### 6. Feature Flags / Gradual Rollout
No mechanism for safely shipping experimental features.

**Suggested system:**
```toml
[features]
semantic_memory = true      # hybrid vector search (now shipped)
mcp_client = false          # wip
streaming_responses = false # wip
```

---

### 7. Multi-Modal Support (Vision)
Claude 3.5 Sonnet and GPT-4o support image input. Praxis has no path for this.

**What's needed:**
- `image_url` or `base64_image` field in tool requests / prompts.
- Attachment type detection (`image/png`, `image/jpeg`).
- Token counting for images (Claude: ~$0.003/image, OpenAI: varies by detail
  setting).

---

### 8. Plugin / Extension System
Right now adding a tool means editing `src/tools/` and recompiling.

**Options:**
- **WASM plugins:** Compile tool logic to `.wasm`, load at runtime via
  `wasmtime`. Sandboxed, portable.
- **Python plugins:** Embed `pyo3` or call subprocesses. Slower but accessible.
- **External MCP servers:** Already covered by item #1.

---

### 9. Advanced Memory (Beyond Embeddings)
The new vector search is a placeholder. Real semantic memory needs:

| Feature | Status | Blocker |
|---------|--------|---------|
| Vector DB (pgvector, Qdrant, Milvus) | Not started | Adds infra dependency |
| Memory graph (entity extraction + KG) | Not started | Needs NER pipeline |
| Episodic replay ("what did I do on Tuesday?") | Partial (session table) | No natural-language query |
| Memory summarisation (condense 10 sessions → 1) | Not started | Summarisation quality |

---

### 10. Operator UI / Dashboard Enhancements
The React dashboard is functional but thin:

- **No real-time logs:** Operator can't see what the agent is doing mid-session.
- **No diff view for approvals:** Can't see what `apply_edit` will change before
  approving.
- **No conversation history:** Can't review past agent reasoning.
- **No model comparison:** Can't A/B test Claude vs GPT-4o on the same task.

---

## 🟡 Nice-to-Have (Lower Priority)

### 11. Local-First / Offline Mode Improvements
Lite mode exists but could be deeper:
- Bundle a small local model (llama.cpp, MLX on macOS) as a fallback.
- Sync sessions to SQLite when offline, push when online.
- P2P operator sync between devices (no central server).

---

### 12. Security Hardening
- **Sandboxed file access:** `chroot` or `landlock` for the agent process.
- **Network egress filtering:** Only allowlisted domains (prevent data
  exfiltration via tool calls).
- **Audit logging to immutable store:** The Merkle trail exists but isn't wired
  into all actions yet.

---

### 13. Developer Experience
- **REPL / debugger:** Step through a session phase-by-phase.
- **Dry-run mode:** Execute a full session without side effects (mock all tools).
- **Snapshot replay:** Re-run a past session deterministically for debugging.

---

### 14. Natural Language Goals → Structured Tasks
Right now goals are markdown files the operator writes. An LLM could:
- Parse vague goals ("fix the auth bug") into structured sub-goals.
- Suggest task ordering based on dependency analysis.
- Auto-generate acceptance criteria.

---

### 15. Cost Optimisation Engine
The token ledger exists but there's no active optimisation:
- Route cheap queries to `gpt-4o-mini`, complex ones to Claude.
- Cache repeated system prompts (prompt caching is wired for Claude but not
  actively managed).
- Compress context windows by summarising old turns.

---

## 🟢 Completed in This Review

| Feature | Files | Status |
|---------|-------|--------|
| Hybrid vector memory search | `memory/vector.rs`, `storage/sqlite/` | ✅ Shipped |
| Lite mode | `src/lite.rs` | ✅ Shipped |
| Merkle audit trail | `src/merkle.rs` | ✅ Shipped |
| Secret zeroization | `src/crypto/zeroize.rs` | ✅ Shipped |
| Multi-provider OAuth | `oauth/copilot.rs`, `providers/mod.rs` | ✅ Shipped |
| Watchdog binary | `src/watchdog/main.rs` | ✅ Shipped |
| CI improvements | `.github/workflows/ci.yml` | ✅ Shipped |
| Frontend enhancements | `frontend/src/pages/` | ✅ Shipped |

---

## Next Recommended Priority

**1. MCP** — Highest ecosystem multiplier. An empty `src/mcp/` is a glaring gap
in an otherwise mature architecture.

**2. Streaming** — Required for MCP server mode (clients expect streaming
responses) and for any serious dashboard UX.

**3. Observability** — Required before production deployments at scale.
