# Praxis

**A standalone, self-evolving personal AI agent framework written in Rust.**

Praxis is designed for anyone to run their own instance of a personal AI agent that learns from them, grows with them, and becomes genuinely irreplaceable over time. It is not a chatbot. It is not a tool. It is an agent that wakes up on its own, pursues goals, builds capabilities, and shapes itself around the specific person running it.

The reference implementation is **Axonix** — a live instance running at [axonix.live](https://axonix.live), built and maintained by the framework's author. Everything in Praxis has been proven in Axonix first.

---

## Reading Guide

This is the canonical design document for Praxis. Notes that previously lived in `Praxis_More.md` are merged here so the architecture, roadmap, and captured additions all live in one place.

If you want the shortest path through the document, read these sections in order:

1. Philosophy
2. The Agent Loop
3. Memory System
4. Context Budget System
5. Tools System
6. Quality Assurance
7. Identity Policy
8. Captured Additions

---

## Origin

Praxis synthesizes the best ideas from several projects:

- **Axonix** (`github.com/coe0718/axonix`) — the reference implementation. 18+ days of live evolution, 86+ sessions, 100% commit rate, 23,000+ lines written autonomously. Proven growth model, memory system, Telegram integration, dashboard, morning brief, prediction tracking, self-assessment. Written in Rust.
- **ClaudeClaw** (`github.com/moazbuilds/claudeclaw`) — proved people want always-on personal agents. Contributed: heartbeat pattern, quiet hours, security levels, voice/image/file attachments, Discord integration, model fallback.
- **earlyaidopters/claudeclaw** — contributed the 5-layer memory architecture concept, which Praxis improves upon significantly.
- **secure-openclaw** (ComposioHQ) — contributed: sender pairing code security model, Signal platform support, multi-provider architecture.
- **claude-caliper** (nikhilsitaram) — contributed: no self-review rule, success criteria JSON per goal, Brier score calibration.
- **superpowers** (obra) — contributed: skill dependency chaining and composable skill resolver.
- **get-shit-done** (gsd-build) — contributed: context handoff notes, dependency-aware wave execution, seed-style future triggers, model profiles, and explicit context-rot prevention.
- **OpenWolf** — contributed: anatomy/file-map indexing, do-not-repeat operational memory, repeated-read detection, bug logging, and token-ledger thinking for long-running Claude sessions.
- **Ralph** — contributed: PRD/story-driven dev loops, append-only operational learnings, sharper AGENTS-style convention capture, and formal stop-condition framing for dev runtimes.
- **tinyagi** (TinyAGI) — contributed: SSE event streaming architecture, TUI dashboard concept.
- **openfang** (RightNow-AI) — contributed: loop guard / circuit breaker pattern, signed tool manifests.
- **claude-code-agents** (undeadlist) — contributed: parallel sub-agent audit patterns.
- **Hermes Agent** — contributed: autonomous skill synthesis in Reflect, progressive skill disclosure, structured operator modeling, and attention to skill portability standards.
- **ZeroClaw** — contributed: thin `reqwest`-first provider adapters, compile-time feature gating, adapter-local auth handling, and an API-key-first public framework stance.
- **OpenClaw** — contributed: `SOUL.md` core identity split, agent-driven skill discovery/install, typing indicators, group activation modes, interactive compaction, first-class model failover, and channel-scoped sandbox thinking.
- **NanoBot** — contributed: treating heartbeat as separate from cron scheduling, plus a central message bus for loose coupling.
- **AstrBot** — contributed: workflow-provider thinking, automatic in-session context compression, clear "Agent Sandbox" framing, and marketplace/spec discipline.
- **PicoClaw** — contributed: runtime steering, hook system design, sub-agent status queries, config sensitivity separation, rule-based model routing, and the idea of a dedicated headless TUI launcher.
- **NanoClaw** (qwibitai) — contributed: credential vault proxy patterns and per-context-group memory/filesystem isolation.
- **GoClaw** (nextlevelbuilder) — contributed: formal evaluate loops, delegation vs handoff distinction, quality gates, prompt caching, credential scrubbing, lane-based scheduling, and env-first onboarding.

---

## Philosophy

**One instance per person.** Praxis is not a shared service. You run it on your own hardware — a VPS, a Raspberry Pi, a home server. Your data stays yours. Your agent shapes itself around you specifically, not around a generic user profile.

Work, personal, and staging instances are valid use cases, but they are modeled as **separate isolated Praxis instances**, each with its own data directory, credentials, memory, and identity surface. What Praxis explicitly rejects is a single blended instance serving multiple people or multiple conflicting operator roles at once. That keeps memory coherent, boundaries legible, and trust local to one operator relationship.

**Provider choice is explicit.** Praxis should be able to route across Claude, OpenAI, and local Ollama depending on operator preference, availability, budget, and privacy constraints. The point is not allegiance to one vendor. The point is keeping the agent alive and trustworthy when one provider changes, rate-limits, or disappears.

Providers are broader than raw model vendors. A Praxis provider may be a direct model API, an OpenAI-compatible surface, a local runner, or an external workflow/orchestration platform when the operator already has one in place.

**Single binary.** `curl` the install script, answer a few questions, and you have a running agent. No Node, no Python, no dependency hell. Heavy or niche integrations should be behind compile-time feature flags so the default binary stays lean.

**Privacy by default.** Every external surface is off or localhost-only until you explicitly opt in.

**Irreplaceable is the goal.** The boss level — the thing every Praxis instance is working toward — is the moment its operator says: *"I couldn't do without this now."*

---

## Tech Stack

| Component | Library |
|---|---|
| Async runtime | `tokio` (full features) |
| Web server (dashboard) | `axum` + `tower-http` |
| Database | `rusqlite` with FTS5 (bundled) |
| HTTP client | `reqwest` |
| Agent core | Praxis-owned traits over swappable provider/runtime backends |
| Serialization | `serde` + `serde_json` + `serde_yaml` |
| Config | `toml` (praxis.toml) |
| Auth (webhook HMAC) | `hmac` + `sha2` |
| TUI | `ratatui` + `crossterm` |
| Crypto | `ed25519-dalek` (tool manifest signing) |
| Scheduling | `tokio-cron-scheduler` |

Single binary distribution. Docker support via `docker-compose.yml`. Runs on Linux, macOS, Raspberry Pi.

---

## Provider Runtime

Praxis should treat provider access as a runtime contract it owns, not as an SDK choice delegated to third-party libraries. The adapter layer should stay thin, explicit, and inspectable.

The default shape:

- one Praxis-owned `Provider` trait implemented over `reqwest`
- one OpenAI-compatible adapter covering the large ecosystem of compatible endpoints
- one native Anthropic adapter for Anthropic-specific features
- optional local or workflow adapters only when the protocol truly differs

This keeps the core small while still supporting a wide provider surface without binding Praxis to a single vendor SDK.

Each adapter owns its own auth complexity. HMAC signing, JWT minting, custom headers, retry semantics, and provider-specific quirks live inside the adapter that needs them, not inside a global shared auth layer.

Failover and routing should be first-class runtime behavior, not just static config. Praxis should support:

- named execution profiles
- rule-based routing for cheap/fast vs deep/reliable work
- automatic failover between preferred and fallback providers
- prompt caching on repeated prefixes where upstreams support it
- cost- and rate-aware selection, ideally backed by a GCRA-style limiter

For a public framework, Praxis should prefer API keys and service credentials over consumer subscription OAuth flows. Subscription OAuth is too policy-fragile to be a core install dependency for something meant to survive provider churn and terms-of-service changes.

Install and container flows should also support env-first onboarding: if standard provider env vars are already present, Praxis should self-configure the relevant routes without forcing an interactive credential setup first.

---

## The Agent Loop

Every Praxis session runs through five phases in order. No phase assumes Git, no phase assumes a developer context. The loop is abstract — what happens in each phase depends on the instance's configuration and tools.

```
wake → orient → decide → act → reflect → sleep
```

Each phase is independently resumable. If Praxis crashes mid-session, it reads a lightweight session state file to pick up from the last completed phase rather than starting over.

### Orient
Load context according to the budget system. Read `SOUL.md`, `IDENTITY.md`, active goals, recent memory, predictions, the structured operator model, and `PATTERNS.md`. Query the analytics table for relevant performance patterns. Consult the codebase anatomy index before opening files so the agent can decide whether a summary is enough or whether a targeted read is justified. Load high-priority operational memory as well: the do-not-repeat register and any known bug-log entries relevant to the current task or goal. Receive any pending messages from messaging platforms. If the instance runs on local hardware, capture a lightweight hardware snapshot as well: CPU load, memory pressure, disk free space, and temperature when available. Know what today looks like before deciding anything.

### Decide
Choose what to work on. Source priority:
1. Incoming user messages (highest — operator spoke)
2. Urgent predictions expiring soon
3. Active goals from GOALS.md
4. Community input (GitHub issues with `agent-input` label, if enabled)
5. Self-directed goals from self-assessment

For any Level 3+ goal, Decide produces a structured plan including assumptions, risks, and explicit success criteria before proceeding to Act. The loop pauses for operator confirmation if the goal's security level requires it.

Every non-trivial choice in Decide writes a **decision receipt**: what was selected, what alternatives were considered, what context was included, what context was dropped by budget, why the choice won, and how confident the agent was. Autonomy without receipts is too opaque to trust.

### Act
Do the work. This is the only phase that touches the outside world. It may involve writing code, calling tools, spawning sub-agents, sending messages, updating files, or posting to social. Act writes a session state checkpoint at each significant step so it can resume after a crash.

The **loop guard** runs throughout Act: a SHA256 hash of recent tool invocations is maintained in the session state file. If the same tool+args hash appears 3+ times consecutively, Act breaks immediately, notifies the operator via messaging, and waits. This prevents runaway tool loops from consuming tokens indefinitely.

For risky operations, Act can enter **sandboxed rehearsal mode** first. Praxis clones the target workspace into a temp directory or disposable container, runs the plan there, captures the intended diff and side effects, and only then decides whether to perform the real action. Rehearsal is mandatory for deploy-like actions unless the tool explicitly opts out.

For complex goals with multiple plausible approaches, Act may also use **speculative execution**: rehearse two or more lightweight branches, compare them against success criteria and trust constraints, and only then commit to the branch most likely to succeed. This is especially useful for dev-focused instances where the cheapest safe move is often "try two narrow approaches in rehearsal, not one large irreversible one."

Long Act phases should remain steerable. At safe checkpoints between tool calls, an operator or supervising system can inject a steering message into the running loop without killing the session. Steering is distinct from ordinary queued messages: it redirects the current run in flight while still preserving receipts, approvals, and auditability.

Within a session, file reads are tracked too. If the same file is requested again without a changed timestamp, narrower symbol target, or new justification, Praxis should warn, reuse the existing excerpt, or fall back to the anatomy summary instead of silently re-reading the whole file.

### Reflect
After Act completes, Reflect runs **before** marking a goal done:
1. Spawn a reviewer sub-agent with a fresh minimal context window
2. Reviewer checks the work against the goal's success criteria JSON (if it exists)
3. If criteria pass → mark complete, capture memory, update files
4. If criteria fail → return goal to active with reviewer's findings attached

Memory capture runs at session end (not per-turn). One extraction pass over the full session transcript. Files updated: GOALS.md, JOURNAL.md, METRICS.md, PATTERNS.md. Analytics rows written to SQLite, including a per-phase token ledger when provider-backed models were used. Reflect is also where Praxis decides whether a session produced a new do-not-repeat item, a reusable bug-log entry, or a postmortem-worthy failure pattern.

After a sufficiently complex successful task, Reflect may also synthesize a reusable `SKILL.md` draft automatically. This is reserved for workflows that were multi-step, reusable, and likely to recur. Auto-synthesized skills should enter the same proposal/review path as other durable capability changes rather than silently activating themselves.

Reviewer context is explicitly capped. A reviewer gets the goal description, success criteria, relevant diffs, tool outputs, the decision receipt, and only the smallest transcript slice needed to verify behavior. It should not receive the full session transcript by default. Hard ceiling: the smaller of 15% of the primary session context budget or 8k tokens.

### Sleep
Wait for the next scheduled wake time. Respect quiet hours. Hand control to the watchdog.

---

## Memory System

Memory is what makes an instance feel like it knows you. It is the most important system in Praxis.

### Design principles

**Capture at session end, not per turn.** After the session completes, one single extraction pass runs over the full session transcript. One API call (Haiku), full context, far better signal than per-turn evaluation. Things mentioned once in passing score low. Things returned to repeatedly score high.

**Two storage tiers:**

`hot` — recent memories, full text, high specificity, FTS5 indexed. TTL of 30 days without access or reinforcement.

`cold` — consolidated insights, shorter, more abstract, weighted higher in search. Synthesized from clusters of hot memories. TTL of 90 days without reinforcement before demotion back to hot.

**Daily consolidation.** Once per day, a consolidation pass looks for clusters of related hot memories and synthesizes them into cold memories. Predictable schedule, not reactive.

**Reinforcement.** When a new hot memory connects to an existing cold one, the cold memory's weight increments rather than storing a duplicate.

**Contradiction detection.** When a new memory directly conflicts with an existing cold one, both are flagged and the conflict surfaces at the next session start for explicit resolution.

**Forgetting.** Hot memories expire after 30 days without access. Cold memories should decay in place first. Demotion back to hot only happens when a once-stable insight becomes stale or uncertain and needs re-validation through fresh evidence, not as the default path for every old cold memory.

**Milestone protection.** Some memories should never be treated like disposable recency facts. Major operator preferences, life events, durable boundaries, and high-importance insights can be marked permanent or archival. These survive normal TTL decay unless explicitly revoked or contradicted.

**Preference graph.** General memory is not enough. Praxis keeps a structured graph of durable operator facts: preferences, routines, constraints, dislikes, and standing instructions. Each node stores confidence, provenance, confirmation history, and whether it is a soft preference or a hard boundary.

**Dialectic operator model.** `PATTERNS.md` remains the human-readable conclusion log, but Praxis should also keep a more active structured operator model across sessions: working hypotheses, confirmed traits, tensions, and counter-evidence. The point is not to psychoanalyze the operator. The point is to give the loop an explicit model it can test, refine, and use when deciding how to help.

**Negative / boundary memory.** "Don't message late", "Never spend money without approval", "Do not touch these files", and "Don't suggest this again" are first-class memories, not just notes buried in a journal. Boundary memories load ahead of ordinary hot/cold recall and can veto plans before Act begins.

**Context-group isolation.** When Praxis operates across multiple group chats, channels, or conversation contexts, each context should get its own short-horizon working-memory lane and optional filesystem sandbox. Shared long-term identity may bridge across them, but working recall should default to isolation instead of bleed-through.

**Annual synthesis.** Once per year, Praxis performs a deep consolidation pass that writes a `YEAR_IN_REVIEW.md` or `BIOGRAPHY.md` style narrative from durable memories, goals, milestones, and patterns. The purpose is not sentimentality; it is preserving long-horizon continuity that should outlive short TTL windows.

**Operational memory.** Praxis also keeps two execution-focused memory surfaces:

- a **do-not-repeat register** for instance-specific mistakes such as "do not rerun migration X without backup" or "do not reread the whole repo when the anatomy summary is enough"
- a **bug log** for known failures, fixes, and workarounds that the instance should check before trying to solve the same class of problem again

These are different from operator preferences. They are lessons about Praxis's own behavior and should load with very high priority whenever a matching task, file, or capability is in play.

### Schema

```sql
CREATE TABLE hot_memories (
  id INTEGER PRIMARY KEY,
  content TEXT NOT NULL,
  summary TEXT,
  importance REAL DEFAULT 0.5,
  tags TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  last_accessed TIMESTAMP,
  access_count INTEGER DEFAULT 0,
  expires_at TIMESTAMP
);

CREATE VIRTUAL TABLE hot_fts USING fts5(content, summary, tags, content=hot_memories);

CREATE TABLE cold_memories (
  id INTEGER PRIMARY KEY,
  content TEXT NOT NULL,
  source_ids TEXT,
  weight REAL DEFAULT 1.0,
  tags TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  last_reinforced TIMESTAMP,
  contradicts TEXT
);

CREATE VIRTUAL TABLE cold_fts USING fts5(content, tags, content=cold_memories);

CREATE TABLE preferences (
  id INTEGER PRIMARY KEY,
  kind TEXT NOT NULL,            -- preference, routine, constraint, dislike, boundary
  statement TEXT NOT NULL,
  confidence REAL DEFAULT 0.7,
  tags TEXT,
  source_ids TEXT,
  hard_boundary INTEGER DEFAULT 0,
  last_confirmed_at TIMESTAMP,
  superseded_by INTEGER
);

CREATE VIRTUAL TABLE preference_fts USING fts5(statement, tags, content=preferences);

CREATE TABLE do_not_repeat (
  id INTEGER PRIMARY KEY,
  statement TEXT NOT NULL,
  tags TEXT,
  source_session_id INTEGER,
  severity TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  expires_at TIMESTAMP
);

CREATE TABLE known_bugs (
  id INTEGER PRIMARY KEY,
  signature TEXT NOT NULL,
  symptoms TEXT NOT NULL,
  fix_summary TEXT NOT NULL,
  tags TEXT,
  source_session_id INTEGER,
  resolved_at TIMESTAMP,
  last_seen_at TIMESTAMP
);

CREATE VIRTUAL TABLE known_bug_fts USING fts5(signature, symptoms, fix_summary, tags, content=known_bugs);
```

### Rust modules
- `memory/capture.rs` — post-session extraction, importance scoring
- `memory/consolidator.rs` — daily consolidation, contradiction detection
- `memory/search.rs` — unified FTS5 search across hot + cold
- `memory/loader.rs` — session startup injection within context budget
- `memory/decay.rs` — TTL enforcement, demotion logic
- `memory/preferences.rs` — preference graph extraction, boundary resolution, confidence updates
- `memory/ops.rs` — do-not-repeat register, known bug log, operational recall

---

## Learning Runtime

Praxis should not only learn from sessions. It should also have a **learning runtime**: a scheduled, low-risk subsystem that gathers knowledge between main sessions.

The learning runtime:

- pulls from operator-approved sources such as RSS feeds, GitHub notifications, changelogs, docs, and selected local project trees
- writes through the same identity and security policies as the main loop
- is read-heavy, not act-heavy
- has its own provider, token, and budget limits
- synthesizes into cold memory, LEARNINGS.md, PATTERNS.md, and proposed future goals

This makes memory active instead of purely reactive. Praxis should be able to get smarter during sleep windows without pretending that unsupervised crawling is the same thing as operator-approved action.

### Hands

A **Hand** is a packaged autonomous capability: a manifest such as `HAND.toml`, a multi-phase prompt or expert workflow, a `SKILL.md` knowledge document, guardrails, and lifecycle metadata. Hands are how repeated opportunities graduate into durable proactive behaviors.

Examples:

- an opportunity miner that scans for repeated friction on a schedule
- a learning hand that ingests approved sources every morning
- a maintenance hand that performs bounded cleanup or verification work without being explicitly prompted

Hands should be schedulable, installable, reviewable, and removable like other signed capabilities. They give Praxis a named format for autonomy that is stronger than "some cron job exists" but narrower than "the whole agent does everything all the time."

---

## Context Budget System

Every session has a hard token ceiling — defaulting to 80% of the model's context window. The remaining budget is allocated via a priority queue:

```toml
[context]
ceiling_pct = 0.80
budget = [
  { source = "identity",     priority = 1, max_pct = 0.05 },
  { source = "active_goals", priority = 2, max_pct = 0.10 },
  { source = "memory_hot",   priority = 3, max_pct = 0.20 },
  { source = "memory_cold",  priority = 4, max_pct = 0.15 },
  { source = "predictions",  priority = 5, max_pct = 0.05 },
  { source = "patterns",     priority = 6, max_pct = 0.05 },
  { source = "journal",      priority = 7, max_pct = 0.10 },
  { source = "tools",        priority = 8, max_pct = 0.05 },
  { source = "task",         priority = 9, max_pct = 0.25 },
]
```

Each source fills in priority order. Sources exceeding their allocation get summarized via a single Haiku call. Lower-priority sources are dropped entirely — never truncated mid-sentence. The agent is told explicitly what was excluded. The system tracks actual usage over time and auto-tunes allocations.

Skills should use **progressive disclosure**. Orient loads a compact skill catalog first — name, description, tags, token estimate, and provenance — and only pulls full `SKILL.md` content on demand when a chosen plan actually needs that skill. This keeps the skill surface discoverable without paying the full token cost up front.

Included and excluded context are written into the session's decision receipt so the operator can audit whether a bad choice happened because the wrong context was loaded or because the reasoning itself was wrong.

### Codebase anatomy index

Praxis maintains a lightweight anatomy index for files it touches regularly. Each record stores:

- file path
- short human-readable description
- rough token estimate
- last modified time
- optional symbol list or tags

The anatomy index is not a replacement for reading files. It is a routing layer that helps Orient decide when a summary is enough and when a targeted excerpt is actually needed.

```sql
CREATE TABLE anatomy_index (
  path TEXT PRIMARY KEY,
  description TEXT NOT NULL,
  token_estimate INTEGER NOT NULL,
  last_modified_at TIMESTAMP NOT NULL,
  symbols_json TEXT NOT NULL DEFAULT '[]',
  tags_json TEXT NOT NULL DEFAULT '[]'
);
```

Anatomy updates should be cheap and automatic by default: refresh on file reads when the timestamp changed, and optionally sweep stale entries during quiet maintenance windows.

### Repeated read detection

Within a session, Praxis records file reads as `(path, mtime, reason)`. If a file is requested again with the same timestamp and no narrower target, the runtime should warn before re-reading it and prefer one of three options:

1. reuse the previous excerpt already in session state
2. fall back to the anatomy summary
3. request a more specific symbol, range, or question

Repeated reads are not forbidden. They are treated as a cost that should be justified.

### Worked example
If the model window is 200k tokens and `ceiling_pct = 0.80`, Praxis may use up to 160k tokens total. The `max_pct` values are percentages of that 160k ceiling, not of the raw 200k model window. So `task.max_pct = 0.25` means the task can consume at most 40k tokens. If higher-priority sources use less than their cap, lower-priority sources may consume the remaining budget, but no source is guaranteed its full allocation.

### Context handoff notes

For long multi-phase goals where context approaches 50% of the model's window, Praxis writes a structured handoff note and opens a fresh context window:

```json
{
  "goal": "G-096: morning brief 7am push",
  "completed": ["added brief.rs module", "wired --brief flag"],
  "remaining": ["add Telegram push at 7am", "write tests"],
  "key_facts": ["TELEGRAM_BOT_TOKEN in .env", "brief.rs line 42 has collect_memory_context()"],
  "do_not_forget": ["tests must cover all three modes: interactive, --prompt, piped"]
}
```

### Interactive and automatic compaction

Manual compaction and automatic compression solve different problems and Praxis should support both.

Manual `/compact` tells Praxis to write a fresh handoff note and reopen a clean context window for the current task. This is an operator-driven reset.

Automatic context compression runs without operator input when in-session context crosses a configured threshold. Praxis summarizes the least-recent or lowest-priority in-flight material into anchor-preserving notes so the session can continue without a hard reset. This is distinct from the 50% handoff-note rule for multi-phase work.

---

## Tools System

Tools are declared in `praxis.toml` and discovered at startup. Three types:

**HTTP tools:**
```toml
[[tool]]
name = "weather"
description = "Get current weather for a location"
type = "http"
method = "GET"
endpoint = "https://wttr.in/{location}?format=j1"
security_level = 1
```

**Shell tools:**
```toml
[[tool]]
name = "deploy"
description = "Deploy the application to production"
type = "shell"
path = "./scripts/deploy.sh"
args = ["{target}"]
security_level = 3
timeout_secs = 60
```

**Internal tools:**
```toml
[[tool]]
name = "memory_search"
description = "Search agent memory for relevant context"
type = "internal"
function = "memory::search"
security_level = 1
```

### Sandboxed rehearsal mode
For tools with significant blast radius, Praxis can invoke a rehearsal wrapper instead of the live tool:

```toml
[[tool]]
name = "deploy"
type = "shell"
path = "./scripts/deploy.sh"
security_level = 4
rehearsal = { mode = "container", image = "praxis-rehearsal:latest" }
```

The rehearsal output includes intended file diffs, outbound requests, destructive operations, and a plain-language summary of expected impact. Approval messages show the rehearsal result first, not just the raw command.

### Security levels
- **Level 1** — agent calls freely, read-only
- **Level 2** — agent calls freely, write operations
- **Level 3** — agent calls but logs every invocation
- **Level 4** — requires user confirmation via messaging platform (60 second timeout)

Any tool at any level can be marked `require_approval = true` to opt into the one-time approval queue. Pending calls visible via `/queue` command.

### Loop guard
SHA256 hash of (tool_name + serialized_args) stored in a ring buffer of the last 10 invocations. If the same hash appears 3+ consecutive times, the tool call is blocked, the session is paused, and the operator is notified with the stuck tool name and arguments.

### Signed tool manifests
Community-shared tools include a signed `manifest.toml` verified at load time using Ed25519. Users install community tools without auditing source code. The community registry only accepts tools with valid signed manifests.

### Skill dependency chaining
Skills in the `skills/` directory can declare dependencies and auto-trigger sequences:

```toml
[[skill]]
name = "ship-feature"
description = "Full TDD feature development cycle"
depends_on = ["write-spec", "write-tests"]
invoke_after = ["code-review"]
```

The skill resolver handles sequencing automatically. The agent invokes a skill by name; the resolver chains the full sequence without the agent having to orchestrate it manually.

### Agent-driven skill discovery
Praxis should support ClawHub-style agent-driven skill discovery and install. The agent can browse a signed registry, evaluate compatibility, and then propose or install a skill package without manual file-copy work.

The registry format should be published, not merely internal. Praxis should stay compatible with emerging community skill portability conventions such as `agentskills.io` where that does not weaken provenance, approval, or sandbox guarantees.

### Hook system
The runtime should expose event-driven hook points at every major stage: LLM call, tool execution, message delivery, approval request, and result ingestion.

Three hook classes matter:

- **Observers** watch passively and emit metrics, traces, or side-channel state.
- **Interceptors** can modify, reroute, or block a step before it continues.
- **Approval hooks** gate sensitive execution with deterministic policy checks.

Hooks complement security levels rather than replacing them. They are the right place for policies such as output scrubbing, extra validation, or channel-specific restrictions.

---

## Quality Assurance

### Goal lifecycle model
Goals are not just active or done. Every goal moves through an explicit lifecycle:

- `proposed`
- `active`
- `blocked`
- `waiting_on_operator`
- `scheduled`
- `cooldown`
- `review_failed`
- `abandoned`
- `complete`

`GOALS.md` stays human-readable, but the canonical structured state lives in a companion metadata file per goal. This lets Praxis reason about blocked work, deferred work, operator dependencies, and repeated failures without flattening everything into one backlog.

Goal metadata supports waiting and dependency-aware behavior:
- `blocked_by` — another goal, external event, or missing tool
- `wait_until` — absolute time before the goal should be reconsidered
- `wake_when` — condition such as webhook received, package delivered, branch merged, or operator reply
- `last_reviewed_at` — when the goal was last reconsidered

This lets Praxis skip unready work without forgetting it, and lets the scheduler wake a goal when the condition actually changes.

### No self-review rule
The agent that performs a task never reviews its own work. This is a hard policy, not a configuration option.

When the primary agent marks a goal complete, Reflect automatically spawns a reviewer sub-agent with a fresh minimal context window, no knowledge of what the primary intended, and the goal's success criteria JSON. If the reviewer finds failures, the goal returns to active with specific findings attached.

### Evaluate loop
Some tasks need more than a one-shot review. For high-stakes generation or design work, Praxis should support a formal generator/evaluator loop:

1. a generator produces output against explicit pass criteria
2. an evaluator reviews it against those criteria
3. failures return with feedback
4. the loop stops after a configurable max round count or a clean pass

This is a named orchestration primitive, not just "spawn a reviewer again."

### Success criteria JSON
Every goal can have a companion criteria file at `goals/criteria/{goal-id}.json`:

```json
{
  "goal_id": "G-096",
  "done_when": [
    "cargo test passes with 0 failures",
    "morning brief fires at 7am via Telegram",
    "METRICS.md updated with new session row"
  ],
  "verify_with": "shell",
  "commands": [
    "cargo test 2>&1 | tail -1",
    "grep 'G-096' GOALS.md | grep '✓'"
  ]
}
```

The reviewer sub-agent runs these deterministically. A goal is not complete until the commands pass.

### Brier score calibration
When a prediction resolves, its Brier score is calculated:

```
brier_score = (forecast_probability - outcome)²
```

Lower is better; 0.0 is perfect. The analytics table stores scores over time. PATTERNS.md captures calibration findings so the agent adjusts future confidence levels based on its own track record.

### Operator-specific eval suite
Praxis includes a permanent eval bank derived from the operator's real life: actual workflows, recurring tasks, messaging preferences, boundaries, and routines. These are not benchmark prompts from the internet; they are regression tests for becoming more useful to one specific person.

Each eval stores:
- the real-world scenario
- the expected behavior
- the forbidden behavior
- the relevant memories or preferences
- whether failure is cosmetic, functional, or trust-damaging

Major self-modification passes and capability changes rerun this suite automatically. If Praxis gets generally "smarter" but worse for its actual operator, the eval suite should catch it.

### Quality gates
Praxis should also support lightweight deterministic quality gates before outputs or tool results become operator-visible.

A gate can:

- pass immediately
- redact sensitive content
- block delivery
- request a retry with structured feedback

This is the right place for credential scrubbing, formatting constraints, boundary enforcement, and other cheap validations that should happen even when a full evaluate loop would be overkill.

---

## Multi-Agent / Sub-Agent Support

**Orchestrator/worker model:**
- Primary agent has identity, memory, goals, full context
- Sub-agents are lightweight: task + relevant tools + memory slice + token budget
- Sub-agents execute and return structured output to primary
- **Sub-agents cannot write to memory or modify identity files** — hard enforcement

**Three primary use cases:**

Research — sub-agent searches web and local memory in parallel, returns a structured brief, exits.

Parallel execution — multiple sub-agents run simultaneously via `tokio::join!`.

Specialized reviewers — reviewer sub-agent spawned by Reflect to verify completed goals. Brief writer sub-agent for morning briefs.

Sub-agents default to Haiku. Promoted to Sonnet when reasoning is required.

Every sub-agent class has a budget envelope. Research and brief agents get task-scoped slices. Reviewer agents are the tightest: they read the goal, criteria, diff, receipts, and verifier outputs first, and only escalate to broader context if verification cannot proceed otherwise.

Delegation and handoff are separate operations. In delegation, the primary agent stays in control and the sub-agent returns a result. In handoff, another agent takes over the session with a routing override and becomes the active responder until control is explicitly returned.

Running sub-agents should expose `spawn_status` so the operator or primary agent can query state mid-flight instead of waiting blindly for completion.

Longer term, delegation links should support directionality and concurrency caps rather than assuming one flat pool of interchangeable workers.

---

## Messaging

Four platforms, identical feature surface:

| Feature | Telegram | Discord | Signal | WhatsApp |
|---|---|---|---|---|
| Text commands | ✅ | ✅ | ✅ | ✅ |
| Voice messages | ✅ | ✅ | ✅ | ✅ |
| Images | ✅ | ✅ | ✅ | ✅ |
| File attachments | ✅ | ✅ | ✅ | ✅ |
| Typing indicators | ✅ | ✅ | ✅ | ✅ |
| Push notifications | ✅ | ✅ | ✅ | ✅ |
| Morning brief | ✅ | ✅ | ✅ | ✅ |
| Official support | ✅ | ✅ | ✅ | ⚠ opt-in |

**Commands available on all platforms:**
- `/ask <prompt>` — send a task or question
- `/run <task>` — spawn a mini-session for a specific job
- `/goal <description>` — add a goal to the backlog
- `/status` — current active goal + last action
- `/brief` — on-demand morning brief
- `/health` — CPU, memory, disk, uptime
- `/queue` — view pending tool approval requests
- `/approve <id>` — approve a queued tool call
- `/reject <id>` — reject a queued tool call
- `/boundaries` — review and update hard limits, disallowed actions, and standing "never do this" rules

`/ask` is synchronous and lightweight: answer this question, clarify this thing, or do a quick one-shot task. `/run` is asynchronous and session-based: take on a bounded job, update state, and potentially continue across multiple loop phases.

**File attachments** — text-readable documents downloaded via the platform's file API, read as UTF-8, injected into the prompt. Cap at 100KB. If a file exceeds the cap, Praxis must refuse or explicitly chunk/summarize it with operator-visible messaging. It must never silently truncate a document and pretend it saw the whole thing.

**Voice messages** — transcribed before being sent to Claude. Requires a Whisper-compatible endpoint (optional).

While a session is actively working, messaging adapters should emit typing or presence indicators where the platform supports them. The goal is to show liveness without turning the agent into a status-spam bot.

Shared channels and group contexts need per-session activation modes such as mention-only, thread-only, or always-listening. Activation mode belongs to the conversation context, not a single global toggle.

### Sender pairing code security
When an unknown sender messages the agent:
1. Send them a one-time 6-digit pairing code
2. Queue their message
3. Ignore all further input from that sender until the operator runs `/approve-sender <code>` from a trusted device

Prevents prompt injection from external parties entirely. Approved senders stored in `praxis.db`. Revocable via `/revoke-sender <id>`.

### Central message bus
Messaging adapters should publish normalized inbound and outbound events onto a central message bus. The agent loop, approval queue, dashboard, typing indicators, and hook system subscribe to that bus rather than calling each integration directly.

Heartbeat remains the proactive wake mechanism. The message bus is the transport spine. Keeping those concerns separate makes the runtime easier to reason about and extend.

---

## Quiet Hours

```toml
[schedule]
cron = "0 */4 * * *"
quiet_hours_start = "23:00"
quiet_hours_end = "07:00"
timezone = "America/Chicago"
```

Sessions during quiet hours are deferred. Messages queued and delivered at session start. Morning brief fires at the first session after quiet hours end.

Heartbeat is not the same thing as cron. Cron expresses scheduled work. Heartbeat is the lightweight proactive-wake and liveness subsystem that can notice stale sessions, pending triggers, or urgent inbox state between scheduled runs.

### Wake-on-intent
Scheduled cadence is the baseline, not the only wake path. Praxis can also wake immediately on approved high-priority triggers such as:
- a trusted operator message marked urgent
- a configured webhook from a project or service
- a goal `wake_when` condition being satisfied

Wake-on-intent must still respect hard boundaries and quiet-hours policy unless the operator explicitly grants emergency override behavior.

### Lane-based scheduler
Praxis should enforce separate concurrency lanes for interactive/operator sessions, sub-agents, background learning, and cron-driven maintenance. A burst of background work must never starve direct operator interaction.

---

## Security Levels

- **Level 1 (Observe)** — read files, search memory, call read-only APIs
- **Level 2 (Suggest)** — write to own files, send messages, call write APIs
- **Level 3 (Act)** — run shell commands, modify project files, deploy
- **Level 4 (Full)** — unrestricted system access, explicit operator consent required

Default is Level 2. Levels 3 and 4 require explicit opt-in during install.

### Channel-scoped sandboxes
Not every surface should get the same execution rights. A primary local shell or explicitly trusted main surface may map to the full host policy, while non-main chat channels should default to narrower tool sets and stronger isolation such as Docker or WASM-backed sandboxes.

This should be a named first-class operator concept: **Agent Sandbox**. Operators should be able to point to it directly, not merely infer that some isolation exists somewhere in the stack.

### Dynamic trust budget
Static security level is only the ceiling. Praxis also maintains a **dynamic trust budget** that expands or contracts within that ceiling based on recent outcomes for similar actions.

Examples:
- repeated successful low-risk file edits can shorten approval friction for similar edits
- reviewer failures, boundary violations, or rehearsal mismatches shrink trust immediately
- no trust score can ever exceed the operator-selected maximum security level
- hard-boundary memories always override trust expansion

Trust is earned per capability class, not globally. Being good at writing docs should not automatically grant more freedom to deploy production systems.

---

## Identity Policy

Praxis should split identity into two layers:

- `SOUL.md` — the stable core identity and non-negotiable self-concept
- `IDENTITY.md` — the evolving working identity, current habits, and interpreted preferences

`SOUL.md` anchors the instance when `IDENTITY.md` learns, adapts, or drifts. The agent reads both every session, but `SOUL.md` is the harder constraint.

**Freely writable by agent:**
`GOALS.md`, `JOURNAL.md`, `METRICS.md`, `LEARNINGS.md`, `PATTERNS.md`, `predictions.json`, `praxis.db`, `CAPABILITIES.md`, `DAY_COUNT`, `goals/criteria/*.json`

**Cooldown-gated (24hr approval window):**
`IDENTITY.md`, `ROADMAP.md`

Agent proposes changes. Operator receives a diff via messaging: *"Your agent wants to update IDENTITY.md. Reply /approve or /reject within 24 hours. No response = approved."*

**Locked (operator only):**
`SOUL.md`, `praxis.toml`, `.env`, tool definitions, security config

Agent cannot modify these. If it wants a new tool or different security level, it writes a proposal to `PROPOSALS.md`.

### Proposal workflow
`PROPOSALS.md` is the human-readable inbox, not the only state store. Each proposal is also mirrored in a structured queue so Praxis can track lifecycle cleanly.

The current repo has a first-pass version of this for learning/opportunity proposals: the structured queue lives in SQLite, `praxis learn accept|dismiss` updates lifecycle state, and `PROPOSALS.md` is rewritten from the queue so the human-readable file never drifts.

Every proposal includes:
- proposal id
- requested change
- rationale
- risks
- affected files, tools, or permissions
- originating session id
- current status: `pending`, `approved`, `rejected`, `expired`, or `implemented`

Operator replies flow back through the same approval surface as other queued actions: `/queue`, `/approve <id>`, and `/reject <id>`. Accepted proposals become explicit goals or queued maintenance work rather than silently applying themselves.

### Capability ledger
`CAPABILITIES.md` is not just a brag sheet. It is a human-readable ledger of what Praxis can actually do, how reliable each capability is, when it last succeeded, what tools or credentials it depends on, and what its known failure modes are.

Each capability entry tracks:
- name and description
- confidence / reliability score
- last verified timestamp
- required tools, services, or secrets
- recent pass/fail streak
- whether the capability is operator-facing, internal, or experimental

Praxis should consult the capability ledger before proposing ambitious actions. If a capability has low reliability or stale verification, the agent should say so instead of bluffing competence.

---

## Growth Model

| Level | Theme | Goal |
|---|---|---|
| 1 | Survive | Don't break. Build trust in own code. |
| 2 | Know Itself | Metrics, self-assessment, goal formation. |
| 3 | Be Visible | Dashboard, streaming, community presence. |
| 4 | Be Useful | Build real tools for the specific operator. |
| 5 | Be Irreplaceable | Anticipate needs. Become something no generic tool could replace. |

**Boss Level:** *"I couldn't do without this now."*

The roadmap is a starting point, not a cage. Instances are allowed — encouraged — to revise it when they discover a better path.

The levels are load-bearing, not decorative:
- Level 1 favors survival and trust-building: conservative security defaults, no autonomous framework mutation, and heavy reliance on review.
- Level 2 unlocks self-assessment, drift tracking, and better internal planning, but still keeps operator-facing ambition narrow.
- Level 3 enables public surfaces and wake-on-intent interfaces because Praxis can explain itself and recover from mistakes.
- Level 4 is where opportunity mining, operator-specific tool building, and proactive maintenance become core behaviors.
- Level 5 is where long-horizon anticipation, annual synthesis, and genuinely irreplaceable operator fit become the organizing goal.

### Opportunity miner
Praxis should actively look for repeated friction in the operator's life: tasks that recur, messages that repeat, goals that keep getting postponed, and annoyances that surface across days. When it detects a pattern, it creates a proposed goal or automation idea rather than waiting to be asked again.

The opportunity miner works from:
- repeated journal complaints
- backlog churn and blocked goals
- recurring message themes
- repeated manual shell or browser workflows
- failed attempts that indicate an unmet missing tool

This is one of the main mechanisms by which Praxis moves from helpful assistant to irreplaceable operator-specific system.

---

## Observability

### Web dashboard
Live dashboard at a configurable URL. Goals, predictions, journal, metrics, session stream, memory context panel, observations browser. Built and maintained by the agent itself. Defaults to localhost-only with auth token.

### TUI dashboard
`praxis tui` — terminal-native live view using `ratatui`. Shows current phase, active goal, recent tool calls, queue, heartbeat, token usage. For headless installs, RPi setups, and SSH sessions. Praxis may also ship a separate launcher-style TUI binary for homelab environments where a dedicated artifact is more ergonomic than a mode flag.

### SSE event streaming
Praxis exposes a standardized Server-Sent Events endpoint at `/events`:

```
agent:orient_start
agent:tool_call {"tool": "weather", "args": {...}}
agent:tool_result {"status": 200}
agent:loop_guard_triggered {"tool": "deploy", "consecutive_count": 3}
agent:decision_receipt_created {"goal_id": "G-096", "confidence": 0.71}
agent:memory_capture {"importance": 0.8}
agent:reviewer_spawned {"goal_id": "G-096"}
agent:goal_complete {"goal_id": "G-096"}
agent:reflect_complete
```

Any frontend subscribes to this stream. The dashboard is just one consumer. New interfaces can be built without touching the agent core.

### Decision receipts
Every meaningful decision is persisted as a structured receipt:

```json
{
  "goal_id": "G-096",
  "phase": "decide",
  "chosen_action": "work on morning brief delivery",
  "rejected_candidates": [
    "reply to low-priority GitHub issue",
    "refactor memory loader"
  ],
  "included_context": ["identity", "active_goals", "memory_hot", "task"],
  "dropped_context": ["journal", "tools"],
  "confidence": 0.71,
  "rationale": "operator request and expiring prediction outweigh self-directed maintenance",
  "approval_required": false
}
```

Receipts power the dashboard, postmortems, and operator trust. If Praxis makes a bad call, the operator should be able to inspect whether it had the wrong information, the wrong priorities, or the wrong confidence.

### Forensics and time-travel replay

Praxis should write a structured phase snapshot at each boundary so a later forensic pass can reconstruct:

- what context was loaded
- what was excluded by budget
- what decision was made
- which tools or providers were invoked
- what the reviewer saw

This is the practical "time-travel debugger" for long-lived agents. When something goes wrong on day 60, the operator should be able to inspect the exact session path rather than reverse-engineering it from a vague journal entry.

### Analytics table

```sql
CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  day INTEGER,
  session_num INTEGER,
  started_at TIMESTAMP,
  ended_at TIMESTAMP,
  tokens_used INTEGER,
  goals_completed INTEGER,
  goals_attempted INTEGER,
  lines_written INTEGER,
  memory_captures INTEGER,
  loop_guard_triggers INTEGER,
  reviewer_passes INTEGER,
  reviewer_failures INTEGER,
  phase_durations TEXT,
  outcome TEXT
);
```

Findings persist in `PATTERNS.md` as durable conclusions injected at session start as high-priority context.

### Token ledger

Session totals are not enough. Praxis should also keep a per-phase, per-provider token ledger so it can answer questions like:

- why was this session expensive?
- which phase burns the most context?
- did failover increase cost or reduce it?
- are repeated reads or oversized reviews wasting budget?

```sql
CREATE TABLE token_ledger (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL,
  phase TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  estimated_cost_micros INTEGER NOT NULL DEFAULT 0,
  recorded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

Argus, drift detection, provider routing, and future spend controls all get sharper when they can reason over this table instead of one blended token count.

### Weekly alignment review
Once per week, Praxis produces a short alignment review for the operator via dashboard and messaging:

- what it learned this week
- which preferences or constraints seem to have changed
- what patterns became stronger or weaker
- which assumptions need confirmation
- what capabilities improved or regressed
- which opportunities it thinks are worth turning into goals next

The weekly review is not a vanity report. It is a deliberate alignment checkpoint so the operator can correct drift before drift becomes identity.

---

## Auto-Update System

Two-process architecture:

**`praxis-watchdog`** — tiny, stable process. Owns the cron schedule, monitors the main process, handles updates. Swaps binaries during the sleep window between sessions, never mid-session.

**`praxis`** — the main binary. Safely updatable because watchdog manages the swap.

```toml
[updates]
channel = "stable"
auto_update = true
notify_before = true
rollback_on_failure = true
```

If a post-update session fails, watchdog rolls back automatically and notifies the operator. Previous binary always kept for one version.

Before swapping to a newly built binary, the watchdog runs a **canary session** in a sandbox. If Praxis cannot complete a basic orient → decide → sleep loop, or if the operator-specific eval suite regresses, the update is rejected automatically.

The watchdog also needs a backstop. It writes a heartbeat file and monotonic timestamp on every healthy cycle. On Linux, a `systemd` unit or cron check should verify that heartbeat and restart the watchdog if it stalls. The goal is not "a perfect watcher of the watcher"; it is making watchdog failure visible and recoverable.

---

## Privacy

Every external surface defaults to off or private:

```toml
[dashboard]
enabled = true
bind = "127.0.0.1"
port = 8080
auth = true
auth_token = ""       # generated at install

[dashboard.public]
enabled = false
domain = ""

[social]
bluesky = false
github_discussions = false

[community]
public_issues = false
```

Auth tokens generated at install using `rand` + `base64`, stored in `.env`, rotatable via `/rotate-token`.

### Secret handling
Praxis should separate non-sensitive config from sensitive material. Committable instance config can live in `praxis.toml`; secrets and other sensitive overrides should live in `.env` or a dedicated security file such as `.security.yml`.

For higher-security installs, Praxis may use a credential vault proxy so agents never hold raw provider keys in prompt context or long-lived memory. Outbound requests pass through a proxy that injects credentials at request time and can enforce per-agent policies or rate limits.

Regardless of storage backend, secrets should be zeroized in memory as soon as practical, and tool or provider outputs should be scrubbed for accidental credentials before they are written into session history.

---

## Install Experience

```bash
curl -sSf https://install.praxis.dev | bash
```

The install script:
1. Checks dependencies (Rust toolchain, Docker optional)
2. Prompts for provider API keys and GitHub token, or auto-detects them from environment variables when available
3. Opens a fast model-powered interview in the terminal (cheap, runs once)
4. The interview asks 6-8 natural conversational questions covering: who you are, what hardware you're running, what you spend too much time on, which messaging platform(s), timezone, security level preference, dashboard visibility, voice transcription
   It also asks for hard boundaries and "never do this" rules during setup so Praxis starts with at least a minimal boundary memory.
5. The model synthesizes the conversation and writes initial files as a JSON blob
6. Script writes `SOUL.md`, `IDENTITY.md`, `GOALS.md`, `ROADMAP.md`, `JOURNAL.md` (day 0 entry), `praxis.toml`
7. Clones the framework repo, runs `cargo build`, sets up cron via watchdog, fires first session

The installer should also support:
- `--dry-run` to show exactly which files, services, cron jobs, Docker assets, and credentials it will touch
- a zero-config home-server mode that chooses sane defaults for Raspberry Pi or Docker-first installs and prints how to change them later
- env-first onboarding that skips interactive provider setup when canonical environment variables are already present

---

## Repository Structure

```
praxis/
├── src/
│   ├── main.rs
│   ├── loop/
│   │   ├── orient.rs          # Context loading, budget allocation
│   │   ├── decide.rs          # Goal selection, plan generation
│   │   ├── act.rs             # Tool invocation, sub-agent spawning
│   │   ├── reflect.rs         # Reviewer spawn, memory capture, file updates
│   │   └── session.rs         # Session state, crash recovery, loop guard
│   ├── memory/
│   │   ├── capture.rs
│   │   ├── consolidator.rs
│   │   ├── search.rs
│   │   ├── loader.rs
│   │   ├── decay.rs
│   │   └── preferences.rs    # Preference graph + boundary memory
│   ├── messaging/
│   │   ├── telegram.rs
│   │   ├── discord.rs
│   │   ├── signal.rs
│   │   ├── whatsapp.rs        # Unofficial, opt-in
│   │   ├── attachments.rs     # Voice, image, file handling
│   │   └── auth.rs            # Sender pairing code system
│   ├── tools/
│   │   ├── registry.rs
│   │   ├── http.rs
│   │   ├── shell.rs
│   │   ├── security.rs        # Security levels, loop guard, approval queue
│   │   ├── manifest.rs        # Ed25519 signed manifest verification
│   │   └── resolver.rs        # Skill dependency chaining
│   ├── agents/
│   │   ├── primary.rs
│   │   └── subagent.rs        # Lightweight worker + reviewer agents
│   ├── context/
│   │   ├── budget.rs          # Token budget, priority queue, handoff notes
│   │   └── anatomy.rs         # File anatomy index + repeated-read awareness
│   ├── identity/
│   │   └── policy.rs          # File tier enforcement, cooldown gating
│   ├── learning/
│   │   └── runtime.rs         # Scheduled source ingestion + synthesis
│   ├── forensics/
│   │   └── replay.rs          # Phase snapshots, replay, postmortem support
│   ├── quality/
│   │   ├── reviewer.rs        # No self-review enforcement
│   │   ├── criteria.rs        # Success criteria JSON loader + runner
│   │   ├── goal_state.rs      # Rich goal lifecycle model
│   │   ├── evals.rs           # Operator-specific eval suite
│   │   └── brier.rs           # Prediction calibration scoring
│   ├── dashboard/
│   │   └── server.rs          # axum server, SSE /events, auth, static files
│   ├── tui/
│   │   └── app.rs             # ratatui TUI dashboard
│   ├── watchdog/
│   │   └── updater.rs
│   ├── scheduler/
│   │   └── quiet_hours.rs
│   └── analytics/
│       ├── sessions.rs
│       ├── patterns.rs
│       ├── receipts.rs        # Decision receipts + weekly alignment review
│       └── tokens.rs          # Per-phase token ledger + provider cost tracking
├── skills/
├── goals/
│   └── criteria/              # Per-goal success criteria JSON files
├── scripts/
│   ├── install.sh
│   ├── evolve.sh
│   └── interview.sh
├── docs/                      # Dashboard static files (agent-maintained)
├── caddy/
├── .env.example
├── praxis.toml.example
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── SOUL.md
├── IDENTITY.md
├── GOALS.md
├── ROADMAP.md
├── JOURNAL.md
├── METRICS.md
├── PATTERNS.md
├── LEARNINGS.md
├── CAPABILITIES.md
├── PROPOSALS.md
└── DAY_COUNT
```

---

## Cargo.toml

```toml
[package]
name = "praxis"
version = "0.1.0"
edition = "2021"
description = "A self-evolving personal AI agent framework"
license = "MIT"

[features]
default = []
skill-creation = []
matrix = []
lark = []
nostr = []
local-multimodal = []

[dependencies]
tokio = { version = "1", features = ["full"] }
axum = "0.8"
tower-http = { version = "0.6", features = ["fs", "cors"] }
tokio-cron-scheduler = "0.13"
rusqlite = { version = "0.32", features = ["bundled"] }
reqwest = { version = "0.12", features = ["json", "multipart"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"
hmac = "0.12"
sha2 = "0.10"
rand = "0.8"
base64 = "0.22"
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
thiserror = "1"
ratatui = "0.28"
crossterm = "0.28"
ed25519-dalek = { version = "2", features = ["rand_core"] }
tokio-stream = { version = "0.1", features = ["sync"] }
zeroize = { version = "1", features = ["zeroize_derive"] }

[dev-dependencies]
tempfile = "3"
tokio-test = "0.4"

[[bin]]
name = "praxis"
path = "src/main.rs"

[[bin]]
name = "praxis-watchdog"
path = "src/watchdog/main.rs"

[profile.release]
opt-level = 2
lto = false
strip = true
```

---

## Build Order

Status values are intentionally rough:

- `Implemented` means the repo has a working first pass today.
- `In progress` means the system exists, but the full design goal is not complete yet.
- `Planned` means the section is still design-only.

| Step | Status | System | Primary modules |
|---|---|---|---|
| 1 | Implemented | Agent loop skeleton | `src/loop/`, `src/scheduler/` |
| 2 | Implemented | Context budget + anatomy index | `src/context/`, `src/loop/orient.rs` |
| 3 | In progress | Memory + operational memory | `src/memory/`, `src/analytics/` |
| 4 | In progress | Tools system | `src/tools/`, `src/agents/` |
| 5 | In progress | Identity policy | `src/identity/`, `PROPOSALS.md` workflow |
| 6 | Implemented | SSE event streaming | `src/dashboard/`, `src/analytics/receipts.rs` |
| 7 | In progress | Messaging layer | `src/messaging/` |
| 8 | Implemented | Quality system | `src/quality/`, `goals/criteria/` |
| 9 | In progress | Analytics + observability + forensics | `src/analytics/`, `src/forensics/`, `src/tui/`, `docs/` |
| 10 | In progress | Trust + opportunity + learning runtime | `src/quality/`, `src/analytics/`, `src/memory/`, `src/learning/` |
| 11 | Planned | Watchdog + auto-update | `src/watchdog/`, `scripts/install.sh` |

This table is intentionally coupled to the repository structure. If the module layout changes, the build order should be updated in the same pull request.

---

## Captured Additions

The following ideas are explicitly captured so they do not get lost. Some are already reflected in the design above; this section is the durable shortlist of what Praxis should absorb next versus what should remain optional until the core is stable.

This section now absorbs the old `Praxis_More.md` notes as well.

Move items upward as they ship:

- `Completed` means the feature exists in the current repo.
- `Adopt Soon` means it should land in a near-term implementation wave.
- `Future / Optional` means it is intentionally deferred.

### Fresh Comparative Notes (April 2026)

- **Hermes Agent** — autonomous skill synthesis in Reflect; progressive skill disclosure; a dialectic operator model more active than `PATTERNS.md`; and `agentskills.io` portability worth watching.
- **ZeroClaw** — pure `reqwest` provider trait; one OpenAI-compatible adapter plus one Anthropic-native adapter; skill creation behind a Cargo feature; adapter-local HMAC/JWT handling; API-key-first public framework stance; and compile-time channel gating.
- **OpenClaw** — `SOUL.md`; agent-driven skill discovery/install; typing indicators; per-session activation modes for group contexts; `/compact`; first-class model failover; and per-channel sandboxing.
- **NanoBot** — heartbeat as a distinct concern from cron, plus a central message bus that channels publish into and the loop subscribes to.
- **AstrBot** — workflow-orchestrator providers; automatic mid-session context compression; "Agent Sandbox" as a named surface; and the value of a real published marketplace/plugin spec.
- **PicoClaw** — steering; hook system with observers, interceptors, and approval hooks; `spawn_status`; sensitive-config separation; rule-based model routing; and a dedicated launcher TUI artifact.
- **NanoClaw** — credential vault proxying and per-context-group memory/filesystem isolation.
- **OpenFang** — `Hand` capability packages; WASM dual-metered sandboxing; secret zeroization; and GCRA rate limiting.
- **GoClaw** — evaluate loop; explicit delegation links and handoff; quality gates; prompt caching; credential scrubbing; lane-based scheduling; and env-var auto-onboarding.

### Completed

- **Portable state export/import** — `praxis export state` and `praxis import` now produce and restore a manifest-versioned state bundle containing the SQLite database, runtime state, config, tools, learning inputs, and core markdown identity files, while re-homing imported config paths to the new data directory.
- **Schema migration policy** — exported state bundles now carry an explicit bundle format version plus the SQLite schema version so portability and restore compatibility are tracked rather than implicit.
- **Audit exports** — `praxis export audit` now writes a human-readable report of recent sessions, approvals, provider usage, memory writes, and recent event logs.
- **Command semantics** — `praxis ask ...` and Telegram `/ask` are now lightweight and stateless, while `praxis run --once` and Telegram `/run` execute real session work with durable state updates.
- **Codebase anatomy index** — file descriptions, token estimates, and timestamps are stored so Orient can reason about files before reopening them.
- **Do-not-repeat register** — operational mistakes are persisted and can be loaded before similar future work.
- **Known bug log** — searchable bug/fix memory now exists for instance-specific operational learning.
- **Repeated-read detection** — sessions can detect and avoid reopening the same file when the anatomy summary is sufficient.
- **Per-phase token ledger** — token and estimated cost usage is stored by phase and provider in SQLite.
- **Cross-session failure clustering** — Argus groups repeated failure outcomes instead of treating bad sessions as isolated.
- **Cross-session pattern mining** — Argus now spots recurring goals or tasks that keep resurfacing across sessions and days.
- **Drift detection** — a first-pass rolling baseline now marks recent quality as stable, regressed, improving, or insufficient-data.
- **Opportunity miner throttle** — opportunity creation is now deduplicated and capped per day/week so the queue stays bounded.
- **Active learning runtime** — `praxis learn run` now ingests local learning sources, appends syntheses to `LEARNINGS.md`, and records learning runs.
- **AGENTS-style pattern capture** — Praxis now seeds an `AGENTS.md` file, loads it during Orient, and provides CLI support for recording workflow notes, gotchas, and handoff guidance future runs should remember.
- **Append-only operational learnings** — `LEARNINGS.md` now acts as a structured append-only log for manual operational notes and automatic learning-source syntheses, so instance discoveries accumulate without silent rewrites.
- **Hierarchical summarization fallback** — oversized context sources now compress into anchor-preserving summaries that try to keep headings, goal IDs, dates, and boundary-style lines instead of bluntly truncating everything.
- **Cross-tool loop detection** — the loop guard now blocks repeating multi-step invocation patterns such as alternating tool thrash, not just identical single-tool repeats.
- **Tool auto-documentation** — `CAPABILITIES.md` now stays synced with installed tool manifests plus approval/execution history, including recent examples, rejection notes, and simple reliability counts.
- **Postmortem generator** — Reflect now appends structured `POSTMORTEMS.md` entries for reviewer failures, eval regressions, and similar bad session outcomes.
- **End-to-end replay testing** — fixture-backed transcript replays now cover stateful foundation and approval flows so regressions can be caught against recorded multi-step sessions.
- **Energy budget / rate-limit budget** — `budgets.toml` now sets explicit ask/run attempt, token, and estimated-cost ceilings, and Praxis blocks extra backend work once a session exhausts them.
- **Local-first model fallback** — an opt-in `agent.local_first_fallback` policy now routes low-risk `ask` and `act` phases through Ollama first, then falls back to configured cloud providers if needed.
- **Adaptive context allocation** — Praxis now records which context sources were actually included in successful or failed sessions and gently reweights future source caps via a portable `context_adaptation.json` state file.
- **File-mutation circuit breaker** — tool approval validation now caps write-path fanout, protected identity-surface writes, and append payload size before a real file mutation can execute.
- **Watchdog heartbeat backstop** — Praxis now writes a runtime heartbeat file, exposes `praxis heartbeat status/check`, and ships a simple external `scripts/check-heartbeat.sh` checker for cron or systemd.
- **Proposal inbox sync** — the opportunity queue now mirrors into `PROPOSALS.md`, and operators can accept or dismiss proposals without the markdown view drifting out of sync.
- **Opportunity-to-goal promotion** — accepting a mined opportunity can now create or reuse a real goal in `GOALS.md`, link the proposal to that goal, and feed the work back into the main loop.
- **Hierarchical goals** — goals can now declare `parent: G-...`, and the planner will prefer unfinished child work before selecting an umbrella parent goal that still has open children.
- **Controlled data-write execution** — approved `praxis-data-write` requests can now execute a real append-only write into allowed Praxis data files instead of always stopping at a stubbed act-phase record.
- **Agent-core dependency hedge** — Praxis now documents the backend ownership boundary and replacement plan in `docs/agent_runtime.md`, making external runtimes adapters rather than core architecture.
- **Model profiles** — Praxis now seeds `profiles.toml` and applies named `quality`, `budget`, or `offline` execution profiles at config-load time so backend routing and context ceiling can shift together.
- **Model transition controls** — Praxis now stores per-provider/model canary results in `model_canary.json`, exposes `praxis canary run/status`, and can freeze remote routes until a passing canary is recorded.
- **Boundary maintenance loop** — Praxis now persists weekly boundary review state, exposes `praxis boundaries show/add/confirm`, and surfaces the recurring "have any hard limits changed?" prompt in status plus Telegram `/boundaries`.
- **Attachment policy** — Praxis now supports `praxis ask --file ...` with explicit `reject`, `chunk`, or `summarize` handling for oversized UTF-8 text attachments, and it never silently truncates what it injects.
- **Cold-memory decay clarification** — stale cold memories now decay in place through the runtime maintenance pass instead of being automatically demoted back to hot memory.
- **Automatic backup snapshots** — Praxis can now create opt-in daily snapshot bundles in `backups/` during normal runs and prune old snapshots according to retention settings in `praxis.toml`.
- **SOUL / IDENTITY split** — `SOUL.md` is written at foundation as the immutable identity anchor (locked, never agent-writable). `IDENTITY.md` is the evolving working identity. `operator_model.json` holds the active dialectic model (working hypotheses, confirmed traits, tensions, counter-evidence) and is freely writable. Both SOUL.md and operator_model are loaded in Orient as `soul` and `operator_model` context sources. SOUL.md is enforced as locked in the file-mutation policy.
- **Formal evaluate loop and quality gates** — `quality::run_evaluate_loop()` is a named orchestration primitive: generator closure → evaluator verdict → feedback loop, up to `max_rounds`. `GatePipeline` runs ordered `QualityGate` checks (Pass/Redact/Block/RetryWithFeedback). Built-in gates: `NonEmptyGate`, `CredentialScrubGate` (prefix-based sk-/sk-ant-/Bearer scrubbing), `MaxLengthGate`, `ForbiddenPhraseGate`. `default_delivery_pipeline()` composes NonEmpty + CredentialScrub for all operator-visible output.
- **Progressive skill loading and synthesis** — `skills/*.md` files carry TOML frontmatter (name, description, tags, token_estimate). `skills::render_catalog()` emits a compact catalog appended to the tools context source so Orient sees all installed skills cheaply. `read_skill_content()` pulls full body on demand. After a successful goal session with ≥3 tool calls, Reflect calls `SkillSynthesizer::maybe_draft()` which writes a timestamped draft to `skills/drafts/` — drafts never activate themselves and require operator review.
- **Thin provider adapter layer** — `ProviderProtocol` enum routes each provider through the correct wire format: Anthropic-native, OpenAI-compatible Chat Completions, or Ollama. Any custom provider name with a `base_url` is dispatched through the OpenAI-compat adapter automatically. Adapter-local auth resolves `<UPPER>_API_KEY` then falls back to `OPENAI_API_KEY`.
- **Route classes and rule-based routing** — `ProviderRoute` now carries an optional `class` (`fast`, `reliable`, `local`). The router selects by class when dispatching: Fast for Orient/ask, Reliable for Decide/Act. Unclassed routes match any class as a fallback.
- **Prompt caching** — Anthropic adapter sends `cache_control: ephemeral` on the system prompt when `agent.prompt_caching = true`, enabling cache-read discounts on repeated Orient/Ask calls.
- **Env-first onboarding** — `ProviderSettings::load_or_default` now auto-adds claude/openai/ollama routes when `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, or `OLLAMA_HOST` are set and no matching route is configured.

### Adopt Soon

- `Completed` **Capability benchmarking** — add recurring capability tests and operator-specific replay/eval sessions to measure usefulness over time.
- `Completed` **Thin provider adapter layer** — keep providers on a Praxis-owned `reqwest` trait with OpenAI-compatible plus Anthropic-native adapters, adapter-local auth handling, runtime failover, and rule-based routing.
- `Completed` **SOUL / IDENTITY split** — add an immutable `SOUL.md` anchor alongside an evolving `IDENTITY.md`, backed by a structured operator model instead of relying only on `PATTERNS.md`.
- `Completed` **Progressive skill loading and synthesis** — load skill metadata first, pull full docs on demand, and let Reflect draft reusable skills after complex successful work.
- `Completed` **Interactive and automatic compaction** — support explicit `/compact` plus threshold-triggered in-session compression as separate mechanisms.
- `Completed` **Formal evaluate loop and quality gates** — make generator/evaluator review loops and deterministic pre-delivery checks first-class runtime primitives.
- `Completed` **Heartbeat / message-bus separation** — keep liveness and proactive wake logic distinct from transport/event distribution.
- `Completed` **Typing indicators and activation modes** — improve messaging UX in shared channels without forcing a single global listen policy.
- `Completed` **Prompt caching and env-first onboarding** — reduce cost on long sessions and make container or CI installs self-configure when credentials already exist.
- `Completed` **Speculative execution** — compare multiple rehearsed branches before committing to the safest or highest-yield act plan.
- `Completed` **Wave execution** — group dependency-aware sub-agent work into parallel waves instead of spawning parallelism ad hoc.
- `Completed` **Context-rot prevention** — make "fit work into clean context windows" a structural rule, not just a good habit.
- `Completed` **Wake-on-intent** — support approved interrupt-style wakes alongside scheduled sessions.
- `Completed` **Reviewer cost guardrails** — keep reviewer context and token ceilings explicit so mandatory review stays affordable.
- `Completed` **Relational memory layer** — typed `MemoryLink` edges (`caused_by`, `related_to`, `contradicts`, `user_preference`, `follow_up`) stored in SQLite; top hot memories expand through links during context load.
- `Completed` **Operator reinforcement commands** — `/memories`, `/reinforce <id>`, `/forget <id>`, `/link <from> <type> <to>` Telegram commands for direct memory management.
- `Completed` **Config sensitivity split** — `security.toml` (gitignored) holds sensitive overrides such as security level; applied on top of committable `praxis.toml` at startup.
- `Completed` **Decision receipts** — every Decide phase writes a structured audit record (reason code, goal, chosen action, context sources, confidence) to a `decision_receipts` SQLite table; recent receipts surface in context; `/decisions` Telegram command for operator review.

### Future / Optional

- **Hybrid semantic retrieval** — optionally layer vectors or semantic search on top of FTS5 once the keyword + preference graph approach proves insufficient.
- **Memory typing** — extend memory with episodic, semantic, and procedural classes, each with different decay and reinforcement rules.
- **Published marketplace / portability standard** — align skill and tool packaging with a real public ecosystem spec, potentially including `agentskills.io` compatibility where it fits Praxis security needs.
- **Conflict workbench** — create `MEMORY_CONFLICTS.md` or an equivalent workflow to surface conflicting memories with evidence and proposed resolution.
- **Persistent context cache** — store a compressed working set from recent sessions for faster warm starts on constrained hardware.
- **Automatic anatomy refresh daemon** — beyond on-demand updates, optionally re-index changed files during idle windows.
- **Community tool registry improvements** — include compatibility metadata, usage examples, and read-only community discovery.
- **Workflow-provider integrations** — add first-class adapters for external orchestrators when operators want Praxis to sit above an existing automation stack.
- **Local multimodal processing** — optional on-device transcription, captioning, or light image understanding for privacy-preserving installs.
- **System anomaly correlation** — track CPU, memory, disk, and load anomalies against reviewer failures and bad outcomes.
- **Per-context-group isolation** — isolate short-horizon memory and filesystem state by conversation or channel rather than sharing one blended working set.
- **Credential vault proxy** — keep raw provider secrets outside the agent runtime entirely and inject them at request time with policy enforcement.
- **Delegation links** — add explicit outbound/inbound/bidirectional agent links with concurrency caps and allow/deny lists.
- **Granular cooldown policies** — different approval windows per file or identity surface, potentially escalating some files to always-explicit approval.
- **Per-channel sandboxing** — let the main operator surface run with fuller host access while non-main channels default to narrower isolated sandboxes.
- **Hand manifests** — formalize installable/schedulable autonomous capability bundles beyond loose skills and cron jobs.
- **Meta-evolution workflow** — let Praxis propose changes to the framework itself via `SELF_EVOLUTION.md`, with heavy approval gating.
- **Irreplaceability score** — track anticipation, follow-through, reliability, and operator dependence as a private metric, not as a vanity metric.
- **Adaptive scheduling** — let wake times and non-urgent session timing learn from actual operator behavior and quiet-hour patterns.
- **Cargo feature modularity** — keep the single binary lean while allowing optional compile-time extras like skill creation, Matrix/Lark/Nostr channels, voice, vector memory, or advanced graph features.
- **Auto-maintained docs** — let Praxis keep public docs and examples current as capabilities mature.
- **Lite mode** — reduce sub-agent usage, tighten budgets, and simplify behavior for Raspberry Pi or low-power installs.
- **Anonymous learning exchange** — possibly allow instances to publish sanitized, non-personal learnings to a shared registry later, but only with strong privacy guarantees.
- **WASM tool runtime** — support ultra-sandboxed community tools without granting broad local execution, ideally with explicit metering and a watchdog against runaway code.
- **OpenTelemetry / Prometheus export** — richer external observability once local analytics and SSE are stable.
- **Local multimodal and local model bundles** — optional heavy extras for privacy-first or travel/offline deployments.
- **Synthetic example generation** — turn high-value learnings into reusable structured examples for future prompt shaping or evaluation.
- **Social runtime** — optional scheduled outward-facing posting or status sharing on behalf of the operator.
- **Property-based and fuzz testing** — apply proptest to core data structures (memory, context budget, goal parsing) and fuzz security-critical components like tool manifest parsing and approval queues.
- **Database connection pooling** — evaluate r2d2 or similar for concurrent access patterns if Praxis ever runs multiple threads or async tasks against SQLite simultaneously.
- **Hot-path performance profiling** — profile clone-heavy paths (context loading, provider request building) under realistic session loads and cache or reduce copies where it matters.
- **Public API documentation** — add doc examples to the major public traits (AgentBackend, MemoryStore, ToolRegistry) so the library surface is self-documenting.
- **Plugin / extension architecture** — make backends, stores, and adapters more pluggable via stable trait objects so third-party extensions can be compiled separately.
- **Multi-node / distributed mode** — design a coordination layer for running multiple Praxis instances against shared goals, with conflict resolution and work partitioning.
- **Security fuzzing** — fuzz the tool manifest parser, approval queue, and boundary enforcement for robustness against malformed or adversarial input.
- **Zero-LLM deterministic mode** — a rule-based operating mode (flagged in `profiles.toml`) that skips all backend calls and uses static decision logic; enables offline maintenance passes, cost-free CI smoke tests, and a guaranteed fallback when providers are unavailable.
- **Git-backed state sync** — optional `praxis git-push` that commits SQLite exports and markdown state files to a git remote, giving operators an immutable audit trail and easy rollback of any session's effects.
- **MCP (Model Context Protocol) native integration** — register Praxis tools and skills as MCP-native resources so other agents and editors can consume them without custom adapters, and let Praxis consume third-party MCP servers as first-class tool sources.
- **Discord and Slack adapters** — extend `src/messaging/` to cover Discord and Slack with the same activation-mode and typing-indicator model as Telegram, reaching operators on their primary platforms.
- **Voice transcript streaming** — pipe real-time speech-to-text (Whisper or equivalent) into the ask and run entry points so operators can interact hands-free from a Pi or mobile device.
- **Serverless / edge entry point** — a minimal stateless Praxis handler compatible with Cloudflare Workers or AWS Lambda for low-frequency, on-demand runs without a persistent process.
- **VS Code ops surface** — lightweight editor integration for status, current goal, and safe run triggers.
- **PRD/story-mode dev runtime** — an optional developer-focused operating mode that works from explicit story state and stop signals.
- **Dedicated TUI launcher** — package a separate headless/SSH-first TUI artifact instead of only exposing terminal UI as a subcommand.

---

## Relationship to Axonix

Axonix is the reference implementation. It runs on dedicated hardware, evolves every 4 hours, and proves out every feature before it ships in Praxis. When Axonix builds something new and it works, that pattern gets extracted into the framework. When Praxis adds something new, Axonix picks it up autonomously in its next session.

---

## What Praxis Is Not

- Not a hosted service. You run it yourself.
- Not a chatbot. It has its own goals and wakes up without being asked.
- Not a coding agent specifically. The loop is abstract.
- Not finished. The framework evolves alongside Axonix.

---

*Praxis. Theory becoming action.*
