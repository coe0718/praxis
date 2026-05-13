# Gap Analysis: Praxis vs. Axon2 & Axonix

Generated: 2026-05-12

**Purpose:** Identify every feature from Praxis's ancestors (Axon2, 69.5K LOC Python; Axonix, 44.5K LOC Rust) that Praxis (34K LOC Rust) is currently missing. This is the shopping list for evolution candidates.

---

## Legend

| Prefix | Source |
|--------|--------|
| A2 | Axon2 only (Python AI swarm OS) |
| AX | Axonix only (Claude-powered self-evolving coding agent) |
| BOTH | Both Axon2 and Axonix have it; Praxis doesn't |
| PRAX | Praxis has it (reference) |

---

## 1. ORCHESTRATION & CONTROL PLANE

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Goal Intake & DAG Planning** | A2 | Natural language goals → structured DAGs with dependency awareness | High |
| **Topology Templates** | A2 | 5 topologies: chain, parallel_worker, debate, review_swarm, fractal_worker — with heuristic selection | High |
| **Self-Healing DAGs** | A2 | Automatic retry on failure + dead-letter parking on exhaustion | Medium |
| **Durable State Machine** | A2 | Postgres-backed workflow/task lifecycle with leases, retry policies, approvals, reconciliation | High |
| **DAG Cycle Detection** | A2/AX (graphed) | Kahn's algorithm topological sort + cycle detection | PRAX (partial — no DAG in Praxis) |
| **Speculative Execution (multi-branch)** | A2 | Start dependent tasks before full dependency chain closes; commit or rollback | PRAX (speculative module exists) |
| **Dry-Run / Simulation Mode** | A2 | Full workflow simulation without side effects | Medium |

## 2. SUB-AGENT & MULTI-AGENT SYSTEMS

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Dedicated Sub-Agent Roles** | AX | 3 built-in: code_reviewer (haiku), community_responder, implementer — each with different model, max turns, and context | High |
| **Sub-Agent Activity Monitor** | A2 | Captures implementer/tester/reviewer activity as trace events | Medium |
| **Multi-pass Logical Sub-Agents** | A2 | SubagentCoordinator for multi-pass reasoning within single trace (impl/tester/reviewer) | Medium |
| **Kanban Board / Worker Pool** | PRAX | Discord-based kanban orchestration | PRAX |
| **Council / Debate** | PRAX | Multi-agent council (Locke, Sable, Maren) | PRAX (broken — infinite loop) |

## 3. MEMORY & KNOWLEDGE SYSTEM — DEEP DIVE

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **3-Tier Memory** | A2 | Agent (private), Project (shared context), Global (system-wide lessons/mistakes) | High |
| **5-Layer Memory** | AX | Capture → Hot Memory (30d) → Reflection → Cold Memory (90d) → Contradiction detection | High |
| **SQLite Structured Memory** | AX | 9+ tables: kv, sessions, goals, predictions, observations, hot_memories, cold_memories, contradictions, embeddings | PRAX (Mnemosyne is different approach) |
| **Semantic Embeddings** | AX | Local Ollama embeddings (nomic-embed-text, 768d). Auto-embeds on insert. Named entity extraction | High |
| **Vector Database** | A2 | Qdrant with named collections per scope. Scalar/binary/product quantization | High |
| **Graph-Backed Memory Governance** | A2 | Apache AGE graph DB (Postgres extension). Causal topology: agents ↔ projects ↔ memories ↔ lessons ↔ mistakes | Very High |
| **Curator Engine** | A2 | Promotes high-quality lessons to `is_core` when mistake graph shows resolved failures | High |
| **Contradiction Detection** | A2/AX | Both implement it — A2 via graph, AX via cold-memory synthesis | High |
| **Stale Memory Detection** | A2 | Age-out without access | Medium |
| **Two-Stage Dedup** | A2 | SimHash pre-filter + Qdrant cosine similarity (0.92 threshold) | Medium |
| **Lesson Storage with Review Workflow** | A2 | Quality-scored, reviewed, core-promoted lessons with review status migration | Medium |
| **KV Memory Store** | AX | Persistent JSON key-value facts. SQLite write-through for durability | Low (Mnemosyne covers this) |
| **Memory Inspector CLI** | A2 | CLI tool for memory health inspection and pruning | Low |

## 4. SELF-IMPROVEMENT & EVOLUTION

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Identity-Driven Autonomy** | AX | Reads IDENTITY.md + ROADMAP.md + GOALS.md every session. Three operating modes: Tool, Assistant, Autonomous | High |
| **Prediction System** | AX | Self-made predictions about outcomes. Tracks resolution, delta, timing. Computes calibration score (hit rate, bias, days early/late) | Very High |
| **Failure Pattern Tracking** | AX | `.axonix/failure_patterns.json` — cross-session failure type log (e.g., FalseCompletion). Most common type surfaced in brief | High |
| **Metrics Tracking** | AX | METRICS.md — ordered, deduplicated rows. Tokens, tests, files changed, lines, committed status. Crash-safe stub rows | High |
| **Self-Assessment** | AX | Evaluates code quality, goal quality, metrics trends, skill freshness each session. Test count discrepancy check (±10 threshold) | High |
| **Learning Runtime** | A2 | Fetch → Synthesize → Curate pipeline. 13 language-specific sources. 7 knowledge source types | Very High |
| **Local Fine-Tuning Pipeline** | A2 | LoRA adapter training + serving. Benchmark-driven eval. Training profiles | Low (requires GPU, big scope) |
| **Synthetic Example Generation** | A2 | Training examples from successful task completions | Medium |
| **Evolution Sandbox** | A2 | Isolated git worktrees via GitWorktreeManager. Validation gates. Autonomous merge/deploy hooks | High |
| **Argus Meta-Agent** | A2 | Weekly swarm performance + cost analysis. Optimization directives for evolution candidate ranking | High |
| **Journal System** | AX | JOURNAL.md — honest per-session entries. No deletion. Archives old entries (keeps last 15). Injects last 2 into system prompt | PRAX (evolution.jsonl is different) |

## 5. LLM ROUTER & PROVIDER MANAGEMENT

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Role-Specific Model Routing** | A2 | 10 distinct model roles: default, architect, reviewer, tester, coder, social, learn, fast, reasoning, self-correction | High |
| **5 Model Categories with Fallback Pools** | A2 | code, reasoning, general, extraction, fast — each with ordered fallback pools | High |
| **Waterfall Fallback** | A2 | 429/503 → tries fallback pool → escalates to paid models as last resort | High |
| **Provider-Level Circuit Breaker** | A2 | Per-provider, atomic token-bucket refill. Source provider circuit breaker → temporary rewrite onto target provider | High |
| **Semantic LLM Response Caching** | A2 | Redis-backed with configurable TTL. Cache-Control breakpoints for providers that support it | Medium |
| **Self-Correction Model Pivot** | A2 | Explicit model escalation on retries or impasses | Medium |
| **Local LLM Profiles** | A2 | Hardware-aware tier defaults (auto/small/balanced/large). Quantization guidance. Auto-optimization | Medium |
| **Budget-Enforced Paid Call Gating** | A2 | Paid models locked behind daily/weekly/monthly USD budget ceilings | High |
| **BPE Token Estimation** | A2 | Fast pre-check before full tiktoken encoding (Rust-native) | Low |

## 6. TELEGRAM / OPERATOR COMMAND INTERFACE

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **22+ Telegram Commands** | A2 | `/help`, `/status`, `/budget`, `/goals`, `/tasks`, `/approvals`, `/approve`, `/deny`, `/diagnostics`, `/pause`, `/resume`, `/goal`, `/memory`, `/learn`, `/deadletter`, `/replay`, `/trace`, `/plugins`, `/chat`, etc. | High |
| **Telegram RBAC** | A2 | viewer/operator/admin per-chat roles. Separate env lists for each role | Medium |
| **Scheduled Digests** | A2 | Daily news (6pm), weekly digest (Sunday 9am), daily learning digest (8am) | Medium |
| **Alert Forwarding** | A2 | Authenticated Alertmanager webhook → Telegram alerts | Medium |
| **Command Model Routing** | AX | `/run` → full model (Sonnet), other commands → Haiku (fast/cheap) | High |
| **Proactive Task Polling** | AX | Polls GitHub for new `agent-input` issues (15-min interval). Sends daily morning brief at 7 AM | High |
| **Conversation Memory (shared)** | AX | Listener ↔ evolve.sh shared conversation state via .axonix/conversation_memory.json | Medium |
| **24/7 Listener Daemon** | AX | Separate Docker container, polls every 2 seconds, handles commands separately from evolve.sh sessions | PRAX (Hermes gateway covers this differently) |

## 7. API & WEB SURFACE

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **REST API (50+ endpoints)** | A2 | Full control plane over HTTP: goals, tasks, traces, agents, memory, budget, models, social, learning, evolution, policy, plugins, webhooks | Very High |
| **Operator Console** | A2 | Rich web dashboard: Mission Control, Policy Lab, Swarm Theater, Knowledge Map, Runtime Matrix, Evolution Lab, Task Inspector | High |
| **Time-Travel Debugger** | A2 | Snapshot playback, durable annotations, trace comparison, failure clusters, postmortems | High |
| **Dashboard (Public)** | AX | `axonix.live` — HTML dashboard built from METRICS.md, GOALS.md, JOURNAL.md | PRAX (Praxis dashboard exists) |
| **Live SSE Stream** | AX | Real-time session output streaming. SSE endpoint | Medium |
| **VS Code Extension** | A2 | Dashboard open, goal submission, Milton chat, approval review | Low |

## 8. OBSERVABILITY

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Prometheus Metrics** | A2 | FastAPI entrypoints, runtimes, task execution, LLM routing, cache, budget, evolution, Argus, diagnostics | High |
| **Distributed Tracing** | A2 | HTTP trace context propagation, Redis queue tracing, LLM calls, runtime phases, plugin polling | High |
| **Grafana Dashboard** | A2 | Pre-configured axon-observability dashboard | Medium |
| **Loki Log Aggregation** | A2 | Cross-service streaming logs with Promtail | Medium |
| **Alertmanager Routing** | A2 | Prometheus alert rules → Telegram via swarm-bot | Medium |
| **Diagnostic Reports** | A2 | On-demand diagnostic findings: stale agents, DLQ clusters, failing/retry-heavy task hotspots, plugin health regressions | High |
| **Health Watch Daemon** | AX | Periodic threshold checks (CPU > 2.0, memory >85%, disk >85%). Telegram alerts with 300s cooldown. Configurable | PRAX (system-health-monitor skill exists) |
| **Meta Health Check** | AX | Self-diagnosis: predictions file freshness, cycle summary freshness, METRICS.md staleness | Medium |

## 9. INTEGRATIONS & EXTERNAL SERVICES

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Plugin Host (10 integrations)** | A2 | GitHub, Uptime, Gmail, Sentry, RSS, Slack, Jira, Confluence, Azure Budget, GCP Budget | Very High |
| **Plugin Lifecycle Manager** | A2 | States: registered → starting → healthy → degraded → disabled → deprecated. Health checks, auto-disable | High |
| **Plugin SDK** | A2 | Polling/webhook base classes, manifest schema, scaffold templates | Medium |
| **Plugin Credential Security** | A2 | Fernet-encrypted storage. Secret refs: env:, file:, secret:, vault:. Rotation audit history | Medium |
| **Signed Webhooks** | A2 | HMAC verification per-plugin. Delivery history tracking | Medium |
| **GitHub Issues Agent-Input** | AX | Fetch `agent-input` labeled issues sorted by 👍 reactions. Auto-acknowledge, auto-close on fix | High |
| **GitHub Discussions** | AX | Auto-post journal entries via GraphQL API | Low |
| **Bluesky Integration** | AX | Posts session announcements via AT Protocol. Post history, count tracking | Medium |
| **Docker Monitoring** | AX | Container health via Docker socket proxy. Alert on non-running containers | PRAX (system-health-monitor) |
| **Caddy Monitoring** | AX | Caddy admin API health check | Low |
| **Pokémon GO Pogo Module** | AX | Events + promo codes from leekduck.com/ScrapedDuck API | Low (leekduck skill exists) |
| **Ollama Local Embeddings** | AX | Local embeddings API for semantic memory search | Medium |

## 10. POLICY, COMPLIANCE & SAFETY

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Centralized Policy Engine** | A2 | 1,017-line engine covering: dispatch gating, evolution gating, tool permissions, model routing constraints, budget ceilings, approval requirements | Very High |
| **Policy Presets** | A2 | `dev`, `staging`, `prod`, `paranoid` with corresponding infra configs | High |
| **Policy Simulation** | A2 | Policy simulation previews without side effects | Medium |
| **Live Operator Overrides** | A2 | DB-backed editable rule authoring with override records and history | Medium |
| **Risk Assessment** | A2 | Risk level classification: critical, infra, doc, test, dependency | Medium |
| **Hardened Sandbox Profiles** | A2 | Capability drop, no-new-privileges, tmpfs, PID/memory/CPU caps, optional gVisor | PRAX (docker_isolation exists) |

## 11. EVENT SYSTEM & COORDINATION

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Redis Streams Transport** | A2 | 9 defined streams with consumer groups. Standardized event envelope: `{event_id, trace_id, event_type, timestamp, version, payload}` | High |
| **Event Sourcing** | A2 | Every transition published to Redis Stream + archived in Postgres for time-travel debugging | High |
| **Dead Letter Queue** | A2 | Hard-failing tasks parked for manual inspection and replay | Medium |
| **Read/Write Plane Separation** | A2 | Milton writes (Postgres state tables), axon-api reads (projection tables). Background projector indexes events | High |
| **Durable Trace Spans** | A2 | Trace spans in Postgres with configurable capture thresholds. OTLP and Jaeger export | Medium |

## 12. CREDENTIAL & SECRETS MANAGEMENT

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **Secret Resolution Chain** | A2 | Direct → `env:` → `file:` → `secret:` → `vault:` with Fernet-encrypted fallback | High |
| **API Key Authentication** | A2 | Fallback chain for dashboard/Milton/swarm-bot keys | Medium |
| **Per-Token RBAC** | A2 | viewer/operator/admin roles on API via `AXON_OPERATOR_TOKENS_JSON` | Medium |
| **Credential Rotation Audit** | A2 | `plugin_credential_rotations` table with digest-only tracking | Low |

## 13. CLI & OPERATOR EXPERIENCE

| Feature | Source | Description | Priority |
|---------|--------|-------------|----------|
| **30+ REPL Commands** | AX | `/help`, `/quit`, `/clear`, `/retry`, `/model`, `/save`, `/status`, `/history`, `/skills`, `/tokens`, `/context`, `/health`, `/brief`, `/watch`, `/lint`, `/review`, `/ssh`, `/issues`, `/comment`, `/respond`, `/memory`, `/predict`, `/predictions`, `/resolve`, `/failures`, `/summary`, `/recap`, `/archive-journal`, `/files` | High |
| **Multi-process Architecture** | AX | Two-process model: evolve.sh sessions + always-on listener daemon sharing workspace/state | Medium |
| **Cron-based Session Scheduling** | AX | `./scripts/evolve.sh` runs every 4 hours. Full lifecycle: identity → assess → plan → implement → journal | High |
| **Git Commit Discipline** | AX | commit-per-change, bodies required, revert-on-broken-build, G-ID + issue refs in bodies | Medium |
| **Single Prompt Mode** | AX | `--prompt / -p` for one-off non-interactive prompts | Low (covered by CLI) |
| **Piped Mode** | AX | stdin pipe → single response | Low |

## 14. PRAXIS-UNIQUE FEATURES (Not in A2/AX)

For completeness — features Praxis has that neither ancestor does:

| Feature | Description |
|---------|-------------|
| **Orient→Decide→Act→Reflect Loop** | Clean 4-phase loop generics (7 type params). Axonix has a cruder loop, A2 has none |
| **MCP Client** | Model Context Protocol — discover + call external tool servers |
| **Hermes Gateway Integration** | Discord, Telegram, CLI as equal platforms |
| **Discord Council** | Multi-agent debate (broken but the architecture exists) |
| **Tool Manifest System** | TOML-based declarative tool definitions with permission levels |
| **WASM Runtime** | WebAssembly module execution |
| **Wave Engine** | Lightweight compute isolation |
| **Evolution Proposals** | Self-proposals with score-gated generation |
| **Session Scoring** | 4-dimension composite: anticipation, follow-through, reliability, independence |
| **Hook System** | Interceptor (can abort phase) + observer (fire-and-forget) patterns |
| **Filesystem Sandbox** | Per-channel path isolation policy |
| **Desktop Control (Deskbrid)** | Multi-DE desktop automation daemon |
| **Multi-Agent Council** | Separate agent profiles with distinct skills/personalities |

---

## Priority Ranking for Implementation

### Tier 1 — Build first (highest value, lowest effort relative to impact):

1. **Prediction System** (AX) — Self-prediction + calibration score. Low code, high autonomy impact
2. **Failure Pattern Tracking** (AX) — Cross-session failure type log. Simple JSON file, high debugging value
3. **Role-Specific Model Routing** (A2) — 10 model roles with fallback pools. Praxis already has multiple provider configs
4. **Secret Resolution Chain** (A2) — env:/file:/secret:/vault: with fallback. Security win, moderate effort
5. **Journal System** (AX) — Per-session journal with injection into next prompt. Low effort, high context quality

### Tier 2 — Core architecture upgrades:

6. **3-Tier Memory** (A2) — Agent/Project/Global scopes. Requires migration from Mnemosyne architecture
7. **Graph-Backed Memory Governance** (A2) — Causal topology via AGE or SQLite alternative
8. **Self-Assessment + Metrics** (AX) — Score-gated self-evaluation each loop iteration
9. **Goal Intake + DAG Planning** (A2) — Structured goal decomposition instead of flat GOALS.md
10. **Policy Engine** (A2) — Centralized decision engine for dispatch, evolution, tools, budget, approvals

### Tier 3 — Integration layer:

11. **Plugin Host** (A2) — 10+ external integrations with lifecycle management
12. **GitHub Agent-Input Issues** (AX) — Issue polling + auto-acknowledge + auto-close
13. **Telegram Command Expansion** (A2) — More bot commands with role-based routing
14. **REST API** (A2) — Expose Praxis loop externally

### Tier 4 — Observability:

15. **Prometheus Metrics** (A2) — Instrument all phases
16. **Event Sourcing** (A2) — Redis streams for phase transitions
17. **Diagnostic Reports** (A2) — Automated RCA

### Tier 5 — Nice-to-have:

18. **Local Fine-Tuning** (A2) — LoRA adapter pipeline (GPU-dependent)
19. **VS Code Extension** (A2) — IDE integration
20. **Time-Travel Debugger** (A2) — Snapshot playback

---

## Interesting Hybrid Opportunities

1. **Merge A2's DAG Planner with Praxis's Tool Manifest system** — Tools declare their dependencies; Praxis auto-plans execution order. Natural DAG from tool manifests.

2. **A2's Argus + AX's Self-Assessment** — Weekly meta-analysis using self-assessment skill evaluation data. Argus generates optimization directives; Self-Assessment provides the raw data.

3. **A2's Policy Engine + Praxis's Sandbox** — Policy presets map to sandbox profiles. "Paranoid" mode = strict sandbox + approval required on all level-2+ tools.

4. **AX's Prediction System + A2's Budget Ceilings** — Predict resource burn vs. outcome. "I predict this refactor will cost $0.50 in API calls but save 10x that in future tokens."

5. **A2's Read/Write Plane + Praxis's Phases** — Orient phase reads from projected state; Act phase writes through event sourcing. Natural fit.

---

## Summary

| Category | A2 Features | AX Features | Praxis Has | Praxis Missing |
|----------|-------------|-------------|------------|----------------|
| Orchestration | 8 | 0 | 1 (speculative) | 7 |
| Memory | 8 | 6 | 2 (hot/cold) | 12 |
| Self-Improvement | 7 | 7 | 1 (evolution proposals) | 13 |
| LLM Routing | 8 | 0 | 0 | 8 |
| Telegram/Commands | 7 | 5 | 1 (basic) | 11 |
| API/Web | 4 | 2 | 1 (dashboard) | 5 |
| Observability | 6 | 2 | 1 (health monitor) | 7 |
| Integrations | 11 | 5 | 0 | 16 |
| Policy/Safety | 5 | 0 | 2 (sandbox, tools) | 3 |
| Event System | 5 | 0 | 0 | 5 |
| Secrets | 4 | 0 | 0 | 4 |
| CLI/Operator | 0 | 8 | 1 (basic CLI) | 7 |

**Total gaps:** ~98 distinct features across both ancestors that Praxis doesn't have yet.
