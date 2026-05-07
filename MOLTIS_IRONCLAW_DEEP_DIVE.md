# Deep Dive: Moltis & IronClaw — The Two Rust Competitors Worth Watching

**Date:** 2026-05-06  
**Author:** Scout  
**Context:** Feature-by-feature comparison of Moltis and IronClaw against Praxis, with architecture details, security analysis, and concrete recommendations.

---

## Part 1: Moltis — The Closest Rust Competitor

**Repository:** [github.com/moltis-org/moltis](https://github.com/moltis-org/moltis)  
**Creator:** Fabien Penso (French engineer, ex-Microsoft)  
**License:** MIT  
**Language:** Rust — ~270K LoC across **59 workspace crates**  
**Latest:** HN front page, active daily development

### Summary

Moltis is building the same thing Praxis is — a persistent personal AI agent server in Rust. But they've made different architectural bets. Moltis prioritizes a rich web UI and voice I/O. Praxis prioritizes a structured agent loop (Orient/Decide/Act/Reflect) with self-evolution, scoring, and kanban.

### Architecture

```
Gateway Server (Axum HTTP/WS)
  └─ Chat Service
       ├─ Agent Runner (LLM loop)
       ├─ Tool Registry
       └─ Provider Registry
  └─ Sessions | Memory | Hooks | Cron
  └─ Channel Adapters (Telegram, Signal, Discord, etc.)
  └─ Web UI (built-in, compiled in binary)
  └─ Voice Service (STT + TTS crate)
  └─ Browser Automation (Docker containers)
```

59 crates organized as:

| Category | Crates | Combined LoC |
|----------|--------|-------------|
| Core runtime | Gateway, Tools, Providers, Agents, Chat, Config | ~105K |
| Channels | Telegram, WhatsApp, Signal, Discord, MS Teams, Matrix, Slack, Nostr | ~34K |
| Web & APIs | Web UI, GraphQL, Webhooks | ~10.8K |
| Extensibility | MCP, MCP Agent Bridge, Skills, Plugins | ~11.5K |
| Memory & Context | Memory, Qmd, Code Index, Projects | ~11.7K |
| Voice & Browser | Voice (TTS/STT), Browser | ~9.2K |
| Auth & Security | Auth, OAuth, Vault, Secret Store, Network Filter, TLS | ~8.5K |
| Scheduling | Cron, CalDAV, Auto-reply | ~4.7K |

### Feature Comparison: Moltis vs Praxis

#### What Moltis Has That Praxis Doesn't

| Feature | Moltis Implementation | Notes |
|---------|----------------------|-------|
| **Voice I/O** | `moltis-voice` crate: 4 TTS providers (ElevenLabs, OpenAI, Piper local, Coqui local) + 7 STT providers (Whisper, Groq, Deepgram, Google, Mistral, Voxtral, SherpaOnnx). Feature-gated behind `voice` cargo flag. RPC methods: tts.convert, tts.setProvider, stt.transcribe, etc. | Praxis has Whisper listed as "not finished yet" (STT-only). This is a complete voice pipeline. |
| **Web UI (setup/management focus)** | `moltis-web` crate — live Settings → Tools inventory, managed deploy keys, host pinning, password/passkey/API key auth, push notifications, mobile PWA. Setup wizard on first run. | Praxis has a full React SPA (`frontend/`) with 20+ pages covering every subsystem (Dashboard, Chat, Sessions, Goals, Memories, Identity, Learning, Tools, Evolution, Score, Tokens, Canary, Delegation, Argus, Forensics, Boundaries, Vault, Config, Agents, Approvals, Plugins). Recharts-based score/token charts, SSE live events, dark theme, collapsible sidebar, keyboard shortcuts, plugin tab system. **Praxis's web UI is deeper per-subsystem; Moltis's focuses more on setup/settings/key management.** |
| **Signal channel** | `moltis-signal` crate — signal-cli daemon with SSE listener | Praxis only has Telegram, Discord, Slack |
| **Matrix channel** | `moltis-matrix` crate — E2EE support | Not in Praxis |
| **MS Teams channel** | `moltis-msteams` crate | Not in Praxis |
| **Nostr channel** | `moltis-nostr` crate | Not in Praxis |
| **WhatsApp channel** | `moltis-whatsapp` crate — Baileys-based | Not in Praxis |
| **Browser automation** | `moltis-browser` crate — runs in isolated Docker container for web automation tasks | Not in Praxis |
| **SSH remote execution** | SSH/node-backed remote exec with managed deploy keys and host pinning in web UI | Not in Praxis |
| **Edit checkpoints** | Auto-snapshots files before agent edits them | Not in Praxis |
| **Context-file threat scanning** | Scans uploaded files for malware before processing | Not in Praxis |
| **Cursor-compatible project context** | Exports project context in Cursor IDE format | Not in Praxis |
| **OpenTelemetry tracing** | OTel tracing pipeline | Praxis has Prometheus only |
| **OpenAI Batch API** | 50% cost reduction for async batch LLM jobs | Not in Praxis |
| **Push notifications** | Mobile push for approvals, alerts, briefings | Not in Praxis |
| **Multi-device sync** | Sync state/memory across machines | Not in Praxis |
| **Passkeys (WebAuthn)** | Passwordless auth for dashboard | Not in Praxis |
| **Embedding cache** | Cache vector embeddings to reduce API calls | Not in Praxis |
| **Pi-inspired self-extension** | Agent creates its own skills at runtime, session branching, hot-reload | Praxis has similar self-evolution but different mechanism |
| **CalDAV integration** | Calendar integration | Not in Praxis |

#### What Praxis Has That Moltis Doesn't

This is equally important — Praxis has features Moltis lacks:

| Feature | Praxis | Moltis |
|---------|--------|--------|
| **Structured agent loop** | Orient → Decide → Act → Reflect → Sleep | Basic agent loop (no equivalent to Orient/Decide/Reflect phases) |
| **Self-evolution** | `evolution.rs` — append-only JSONL proposals, approval lifecycle, auto-proposal from session failures | Has "Pi-inspired self-extension" but no structured evolution system |
| **Scoring system** | `score.rs` — 4-dimension composite (anticipation, follow-through, reliability, independence) | No scoring |
| **Kanban board** | `src/kanban/` — SQLite store, CLI, dispatcher, worker pattern | No kanban |
| **Curator** | `src/curator/` — run_cycle wired into execute_reflect | No curator |
| **A2A Sync** | `src/a2a/` — inter-agent communication | No equivalent |
| **LanceDB vector memory** | LanceDB-backed long-term memory with semantic recall | SQLite + FTS only (no dedicated vector DB) |
| **Plugin marketplace** | Remote discovery + install of plugins | Plugin support but no marketplace |
| **Skills Hub** | Load catalog, fetch remote catalog, install from URL + CLI | Skills support but no hub |
| **i18n** | 9 languages | Single language |
| **Synthetic examples** | Training triples → evals/examples.jsonl | No synthetic eval system |
| **Anomaly detection** | SystemSnapshot → system_anomalies.jsonl | No anomaly system |
| **Argus (reviewer)** | Per-session reviewer with quality gate | No reviewer system |
| **Model canary** | Freeze on model regression | No canary system |
| **Git state sync** | Auto-commit session state to git repo | No git integration |
| **Tool approval queue** | Required levels, cooldowns, circuit breakers | Simpler approval model |

**Bottom line:** Moltis has a better web UI and voice pipeline. Praxis has a more advanced agent runtime with self-evolution, scoring, kanban, and quality systems. The two projects are surprisingly complementary.

---

## Part 2: IronClaw — The Security-First Agent OS

**Repository:** [github.com/nearai/ironclaw](https://github.com/nearai/ironclaw)  
**Organization:** NEAR AI (AI research arm of NEAR Protocol)  
**Lead:** Yuri Polushkin (74+ commits/week)  
**License:** MIT / Apache 2.0  
**Language:** Rust (~150K LoC)  
**Storage:** PostgreSQL 15+ with pgvector  
**Latest:** Engine v2 in development

### Summary

IronClaw is the security-focused alternative. It's built by NEAR AI (blockchain-adjacent, but the project stands on its own merits). Their signature innovation is the **WASM sandbox** — every tool runs in an isolated WebAssembly container with capability-based permissions. This is the single most differentiating feature in the entire Claw ecosystem.

### Architectural Philosophy

IronClaw's architecture centers on a **two-layer system**:

```
┌─────────────────────────────────────────────────────────────┐
│ Python Layer (engine v2 — self-modifiable orchestrator)     │
│  Monty VM embedded in Rust                                  │
│  Orchestration logic, tool dispatch, output formatting      │
│  Self-improvement missions, prompt overlays                 │
├─────────────────────────────────────────────────────────────┤
│ Rust Layer (stable kernel — rarely changes)                 │
│  LLM backend, tool execution, safety/policy, persistence    │
│  WASM sandbox, credential injection, leak detection         │
│  Thread management, capability lease management             │
└─────────────────────────────────────────────────────────────┘
```

The engine v2 replaces ~10 fragmented abstractions (Session, Job, Routine, Channel, Tool, Skill, Hook, etc.) with 5 primitives:

| Primitive | Replaces | Purpose |
|-----------|----------|---------|
| **Thread** | Session + Job + Routine + Sub-agent | Unit of work with lifecycle, parent-child tree, capability leases |
| **Step** | Agentic loop iteration + tool calls | Unit of execution (one LLM call + action executions) |
| **Capability** | Tool + Skill + Hook + Extension | Unit of effect (actions + knowledge + policies) |
| **MemoryDoc** | Workspace memory blobs | Unit of durable knowledge (summaries, lessons, skills) |
| **Project** | Flat workspace namespace | Unit of context (scopes memory, threads, missions) |

### Feature Comparison: IronClaw vs Praxis

#### What IronClaw Has That Praxis Doesn't

| Feature | IronClaw Implementation | Impact | Est. Effort to Add to Praxis |
|---------|------------------------|--------|-------------------------------|
| **WASM sandbox for tools** | Tools compiled to .wasm, executed via wasmtime. Fuel metering (default 100M instructions), memory limits (16MB), rate limiting. `capabilities.json` declares network/filesystem/credential access. Network requests route through proxy for credential injection and domain validation. | **Highest.** This is the signature innovation in the ecosystem. Makes Praxis the only agent with WASM-isolated tools. | ~1-2 months |
| **WASM sandbox pipeline** | `WASM → Allowlist Validator → Leak Scan → Credential Injector → Execute → Leak Scan → WASM output` | Foundation for all tool security | Part of above |
| **Credential leak detection** | Scans both requests AND responses for secret exfiltration. Credentials injected at host boundary — WASM module never sees raw values. | **High.** Prevents tool-based data exfiltration. | ~1 week |
| **Prompt injection detection** | Pattern-based detection, content sanitization, policy rules with severity levels (Block/Warn/Review/Sanitize), tool output wrapping | **High.** Makes Praxis demonstrably safer. | ~1 week |
| **Endpoint allowlisting** | HTTP requests only to explicitly approved hosts/paths. SSRF protection built-in. | **Medium.** Praxis has path-based sandboxing for files but no network allowlisting. | ~3 days |
| **Routines engine** | Beyond basic cron — event-driven triggers, webhook-reactive, self-healing background workers with heartbeat monitors. Two layers: Scheduler (parallel jobs w/ priorities) + Routines Engine (cron/event/webhook). | **Medium.** Praxis cron is solid but not event-driven/self-healing. | ~2 weeks |
| **OpenAI-compatible API** | `/v1/chat/completions`, `/v1/models`, `/v1/embeddings` with per-request model override. Other tools can use IronClaw as an LLM backend. | **High.** Exposes Praxis as a service. | ~1 week |
| **PostgreSQL + pgvector** | Full PostgreSQL 15+ + pgvector extension for hybrid search with Reciprocal Rank Fusion (RRF). AES-256-GCM encryption. | **Medium.** Praxis uses SQLite + LanceDB. PostgreSQL is heavier but more queryable. Not necessarily a replacement. | N/A (architectural) |
| **Reciprocal Rank Fusion** | Combines vector (pgvector) + full-text (PostgreSQL FTS) rankings for better hybrid search quality. | **Low.** Praxis has separate hot/cold memory. RRF is a ranking algorithm improvement. | ~3 days |
| **Dynamic WASM tool building** | User says "create a tool that does X" → IronClaw generates WASM tool from NL description. Installed into registry. | **Medium.** Powerful but complex. | ~2 weeks |
| **Configuration hot-reload** | Change config without restart | **Low effort, high QoL.** | ~2 days |
| **Hot-reload plugins** | Drop in new WASM tools/channels without restarting | **Medium.** | ~1 week |
| **Self-repair / heartbeat** | Automatic detection and recovery of stuck operations, proactive background execution for monitoring | **Medium.** | ~1 week |
| **Learning missions** (engine v2) | Event-driven: Self-improvement (fired on thread completion with trace issues), Skill repair (fired when skill usage suggests issues). Graduated risk levels. | **Medium.** Praxis has learning system but more structured. The event-driven model is interesting. | ~2 weeks |
| **Formal verification** | Critical security paths formally verified | **Low.** Niche value for this project's stage. | High effort |
| **Multi-agent routing** | Route to different agents per workspace | **Low-medium.** Praxis has A2A sync but different approach. | ~1 week |
| **Diagnostics export** | Sanitized logs/status/health/config/stability snapshots for debugging | **Low effort.** Nice for support. | ~2 days |
| **Tailscale integration** | Secure mesh networking for multi-machine deployments | **Low.** Niche. | ~1 week |
| **Bonjour/mDNS discovery** | LAN discovery | **Low.** Niche. | ~3 days |
| **APNs push pipeline** | Wake disconnected iOS nodes via push notifications | **Low.** Moltis has push too. | ~1 week |
| **Stability snapshots** | Default-on stability recording with event-loop delay/CPU snapshots | **Low effort.** | ~2 days |

#### What Praxis Has That IronClaw Doesn't

Same list as Moltis comparison — Praxis's structured agent loop, self-evolution, kanban, curator, scoring, i18n, LanceDB, etc. are all unique to Praxis in this comparison.

### WASM Sandbox — The Deep Dive

This is IronClaw's killer feature. Here's exactly how it works:

**Execution flow:**
```
LLM selects a tool
  → IronClaw loads .wasm module (from cache or disk)
  → Validates capabilities.json against policy
  → Creates wasmtime instance with fuel metering (100M inst default)
  → Creates memory-limited sandbox (16MB default)
  → Tool executes inside wasmtime
  → Network requests route through network proxy
    → Proxy validates domain against allowlist in capabilities.json
    → Proxy injects credentials from encrypted vault (module never sees them)
    → Proxy enforces per-tool rate limits (e.g. 60 req/min)
  → Filesystem access through declared paths only
  → Output passes through leak detector
  → Result returned to LLM
```

**capabilities.json** (declares what a tool is allowed to do):
```json
{
  "name": "my-tool",
  "network": {
    "allowed_hosts": ["api.example.com"]
  },
  "filesystem": {
    "read": ["/workspace/data/*"],
    "write": ["/workspace/output/*"]
  },
  "credentials": [{
    "name": "example_api_key",
    "inject_as": "Authorization",
    "format": "Bearer {value}"
  }],
  "rate_limit": {
    "requests_per_minute": 60
  }
}
```

**Key design choice:** Credentials are NEVER passed into WASM module memory. The module makes an unauthenticated HTTP request, the proxy intercepts it and injects the auth header. The module cannot exfiltrate secrets because it never possesses them.

**Caching:** WASM modules compiled by wasmtime on first load. Subsequent loads use compiled artifact. Cache at `~/.ironclaw/wasm-cache/`.

**Discovery:** Tools discovered at startup from `~/.ironclaw/tools/` (user-installed) and `/tools/` (per-workspace).

---

## Part 3: Feature Migration Assessment

### What Praxis Should Borrow from Moltis (in priority order)

| Priority | Feature | Why | Technical Approach for Praxis |
|----------|---------|-----|------------------------------|
| 1 | **Wire up sub-agent delegation** | Store exists (`src/delegation.rs`), Act phase just needs to use it. Lowest effort, highest leverage. ~3 days. | Implement `DelegationTool` in tools registry that calls `delegation_store.send_work()` |
| 2 | **Voice I/O: complete the pipeline** | Whisper STT is "not finished yet." Adding TTS makes it useful. The provider pattern (like Praxis provider registry) maps cleanly to TTS/STT providers. ~2 weeks. | Create `PraxisVoiceProvider` trait with TtsProvider + SttProvider subtraits. Feature-gate behind `voice` flag. |
| 3 | **OpenTelemetry tracing** | Praxis has Prometheus but no distributed tracing. OTel is the standard. ~1 week. | Drop in `opentelemetry` + `opentelemetry-otlp` crates. Add tracing spans to loop phases. |
| 4 | **Embedding cache** | LanceDB calls add latency. Simple LRU cache reduces API calls. ~1 day. | Add `moka` or `lru` cache in front of LanceDB memory queries. |
| 5 | **Context-file threat scanning** | Security feature — scan uploaded files before agent processes them. ~1 week. | Add optional `ThreatScanner` trait with `clamav` or `yara` backends. |
| 6 | **OpenAI Batch API** | 50% cost reduction. Praxis provider abstraction makes this natural. ~3 days. | Add BatchApi trait to provider registry, batch non-urgent LLM calls. |
| 7 | **Signal/Matrix/MS Teams channels** | Growing demand for encrypted and enterprise messaging. ~1 week each. | Follow existing messaging platform pattern (Platform trait + inbound polling). |

### What Praxis Should Borrow from IronClaw (in priority order)

| Priority | Feature | Why | Technical Approach for Praxis |
|----------|---------|-----|------------------------------|
| 1 | **OpenAI-compatible API endpoint** | Other tools/agents can use Praxis as an LLM backend. Exposes Praxis as infrastructure. ~1 week. | Create `/v1/chat/completions` + `/v1/models` endpoints on the Axum dashboard server. Route through Praxis provider registry with model mapping. |
| 2 | **Hot-reload config** | Stop-start cycle is friction. ~2 days. | Add inotify/kqueue watcher on `praxis.toml`, reparse on change, update AppConfig in-memory. |
| 3 | **Prompt injection detection** | Practical security differentiator. ~5 days. | Add `PromptSanitizer` trait with pattern-matching backend. Hook into message processing pipeline. |
| 4 | **Credential leak detection** | Catch leaked secrets before they reach user. ~1 week. | Add regex-based scanner for common secret patterns (api_key, sk-, etc.) in tool output. |
| 5 | **WASM sandbox for tools** (long-term) | The ecosystem's most differentiating feature. Would make Praxis the only Rust agent with WASM-isolated tools. ~1-2 months. | Add `wasmtime` as optional dep. Create `WasmTool` variant in ToolKind. Implement host functions (workspace_read/write, log, timestamp). Add capabilities.json parsing. |
| 6 | **Backup archives** | `praxis backup` for portable snapshots. ~2 days. | Use existing state export infrastructure + zip compression. |
| 7 | **Diagnostics export** | Sanitized support bundle. ~2 days. | Collect logs/stats/config into sanitized archive. |

### What Praxis Should NOT Borrow

| Feature | From | Reason |
|---------|------|--------|
| PostgreSQL + pgvector | IronClaw | Praxis SQLite + LanceDB is lighter and sufficient. PostgreSQL adds ops overhead. |
| Embedded Python (Monty VM) | IronClaw | Praxis should stay pure Rust. Python dependency in a Rust agent is a tax. |
| Engine v2 two-layer design | IronClaw | Clever but adds complexity. Praxis's pure-Rust approach is simpler and more auditable. |
| CalDAV integration | Moltis | Niche. Google Meet integration already exists. |
| WASM channels | IronClaw | Over-engineered. Native Rust channel crates are fine. |
| Hot-reload plugins | IronClaw | Nice but adds significant complexity for something that happens rarely. |

---

## Part 4: The Strategic Picture

### Where Praxis Wins

- **Full React SPA web UI** — 20+ pages covering every subsystem (Dashboard, Chat, Sessions, Goals, Memories, Identity, Learning, Tools, Evolution, Score, Tokens, Canary, Delegation, Argus, Forensics, Boundaries, Vault, Config, Agents, Approvals, Plugins). Recharts charts, SSE live events, dark theme, plugin tabs. Moltis has a nicer setup wizard and settings UI; Praxis has deeper per-subsystem pages.
- **Structured agent loop** (Orient/Decide/Act/Reflect) — nobody else has this structured approach
- **Self-evolution + scoring + curator** — the learning pipeline is more mature than any competitor
- **Kanban + A2A sync** — unique among Rust agents
- **LanceDB memory** — proper vector DB, not just SQLite FTS
- **i18n, synthetic evals, anomaly detection, model canary** — depth features no competitor matches
- **Pure Rust, no embedded interpreters** — simpler, faster, more auditable

### Where Praxis Lags

- **Web UI setup/management** — Moltis has a first-run setup wizard, live Settings → Tools inventory, deploy key management, host pinning, and mobile PWA. Praxis's SPA is deeper for subsystem-aware pages but lighter on the admin/setup side.
- **No voice I/O** — Moltis has 11 providers (4 TTS + 7 STT)
- **Fewer messaging channels** — Moltis has 9, Praxis has 3 (Telegram, Discord, Slack)
- **No WASM sandbox** — IronClaw's signature feature
- **No OpenAI-compatible API** — can't serve as infrastructure
- **No prompt injection/credential leak detection** — security differentiators
- **No browser automation** — practical feature for real use

### Recommendations

1. **Short-term (this week):** Wire up delegation. Hot-reload config. Embedding cache. Backup archives. Diagnostics export.
2. **Medium-term (this month):** OpenAI-compatible API. Complete voice pipeline. Prompt injection + credential leak detection. Signal channel.
3. **Long-term (this quarter):** WASM sandbox feasibility study. Browser automation. Full web UI assessment.

The WASM sandbox is the single most differentiating feature in the entire ecosystem — but it's also the hardest. IronClaw spent ~3 months on it. Praxis could potentially do it faster since the tool registry is already mature.

---

## Sources

- [Moltis GitHub](https://github.com/moltis-org/moltis)
- [Moltis Homepage](https://moltis.org/)
- [Moltis Voice Documentation](https://docs.moltis.org/voice.html)
- [IronClaw GitHub](https://github.com/nearai/ironclaw)
- [IronClaw Feature Parity Matrix](https://github.com/nearai/ironclaw/blob/main/FEATURE_PARITY.md)
- [IronClaw WASM Tools Documentation](https://docs.ironclaw.com/capabilities/sandboxed-tools)
- [IronClaw Engine v2 Architecture](https://github.com/nearai/ironclaw/blob/main/docs/internal/engine-v2-architecture.md)
- [IronClaw Research — Ry Walker](https://rywalker.com/research/ironclaw)
- [IronClaw Analysis — Jimmy Song](https://jimmysong.io/ai/ironclaw/)
- [Moltis Architecture — Fabien Penso](https://pen.so/2026/02/12/moltis-a-personal-ai-assistant-built-in-rust/)
