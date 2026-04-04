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

---

## Philosophy

**One instance per person.** Praxis is not a shared service. You run it on your own hardware — a VPS, a Raspberry Pi, a home server. Your data stays yours. Your agent shapes itself around you specifically, not around a generic user profile.

Work, personal, and staging instances are valid use cases, but they are modeled as **separate isolated Praxis instances**, each with its own data directory, credentials, memory, and identity surface. What Praxis explicitly rejects is a single blended instance serving multiple people or multiple conflicting operator roles at once. That keeps memory coherent, boundaries legible, and trust local to one operator relationship.

**Provider choice is explicit.** Praxis should be able to route across Claude, OpenAI, and local Ollama depending on operator preference, availability, budget, and privacy constraints. The point is not allegiance to one vendor. The point is keeping the agent alive and trustworthy when one provider changes, rate-limits, or disappears.

**Single binary.** `curl` the install script, answer a few questions, and you have a running agent. No Node, no Python, no dependency hell.

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

## The Agent Loop

Every Praxis session runs through five phases in order. No phase assumes Git, no phase assumes a developer context. The loop is abstract — what happens in each phase depends on the instance's configuration and tools.

```
wake → orient → decide → act → reflect → sleep
```

Each phase is independently resumable. If Praxis crashes mid-session, it reads a lightweight session state file to pick up from the last completed phase rather than starting over.

### Orient
Load context according to the budget system. Read identity, active goals, recent memory, predictions, and PATTERNS.md. Query the analytics table for relevant performance patterns. Consult the codebase anatomy index before opening files so the agent can decide whether a summary is enough or whether a targeted read is justified. Load high-priority operational memory as well: the do-not-repeat register and any known bug-log entries relevant to the current task or goal. Receive any pending messages from messaging platforms. If the instance runs on local hardware, capture a lightweight hardware snapshot as well: CPU load, memory pressure, disk free space, and temperature when available. Know what today looks like before deciding anything.

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

Within a session, file reads are tracked too. If the same file is requested again without a changed timestamp, narrower symbol target, or new justification, Praxis should warn, reuse the existing excerpt, or fall back to the anatomy summary instead of silently re-reading the whole file.

### Reflect
After Act completes, Reflect runs **before** marking a goal done:
1. Spawn a reviewer sub-agent with a fresh minimal context window
2. Reviewer checks the work against the goal's success criteria JSON (if it exists)
3. If criteria pass → mark complete, capture memory, update files
4. If criteria fail → return goal to active with reviewer's findings attached

Memory capture runs at session end (not per-turn). One extraction pass over the full session transcript. Files updated: GOALS.md, JOURNAL.md, METRICS.md, PATTERNS.md. Analytics rows written to SQLite, including a per-phase token ledger when provider-backed models were used. Reflect is also where Praxis decides whether a session produced a new do-not-repeat item, a reusable bug-log entry, or a postmortem-worthy failure pattern.

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

**Negative / boundary memory.** "Don't message late", "Never spend money without approval", "Do not touch these files", and "Don't suggest this again" are first-class memories, not just notes buried in a journal. Boundary memories load ahead of ordinary hot/cold recall and can veto plans before Act begins.

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

---

## Messaging

Four platforms, identical feature surface:

| Feature | Telegram | Discord | Signal | WhatsApp |
|---|---|---|---|---|
| Text commands | ✅ | ✅ | ✅ | ✅ |
| Voice messages | ✅ | ✅ | ✅ | ✅ |
| Images | ✅ | ✅ | ✅ | ✅ |
| File attachments | ✅ | ✅ | ✅ | ✅ |
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

### Sender pairing code security
When an unknown sender messages the agent:
1. Send them a one-time 6-digit pairing code
2. Queue their message
3. Ignore all further input from that sender until the operator runs `/approve-sender <code>` from a trusted device

Prevents prompt injection from external parties entirely. Approved senders stored in `praxis.db`. Revocable via `/revoke-sender <id>`.

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

### Wake-on-intent
Scheduled cadence is the baseline, not the only wake path. Praxis can also wake immediately on approved high-priority triggers such as:
- a trusted operator message marked urgent
- a configured webhook from a project or service
- a goal `wake_when` condition being satisfied

Wake-on-intent must still respect hard boundaries and quiet-hours policy unless the operator explicitly grants emergency override behavior.

---

## Security Levels

- **Level 1 (Observe)** — read files, search memory, call read-only APIs
- **Level 2 (Suggest)** — write to own files, send messages, call write APIs
- **Level 3 (Act)** — run shell commands, modify project files, deploy
- **Level 4 (Full)** — unrestricted system access, explicit operator consent required

Default is Level 2. Levels 3 and 4 require explicit opt-in during install.

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

**Freely writable by agent:**
`GOALS.md`, `JOURNAL.md`, `METRICS.md`, `LEARNINGS.md`, `PATTERNS.md`, `predictions.json`, `praxis.db`, `CAPABILITIES.md`, `DAY_COUNT`, `goals/criteria/*.json`

**Cooldown-gated (24hr approval window):**
`IDENTITY.md`, `ROADMAP.md`

Agent proposes changes. Operator receives a diff via messaging: *"Your agent wants to update IDENTITY.md. Reply /approve or /reject within 24 hours. No response = approved."*

**Locked (operator only):**
`praxis.toml`, `.env`, tool definitions, security config

Agent cannot modify these. If it wants a new tool or different security level, it writes a proposal to `PROPOSALS.md`.

### Proposal workflow
`PROPOSALS.md` is the human-readable inbox, not the only state store. Each proposal is also mirrored in a structured queue so Praxis can track lifecycle cleanly.

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
`praxis tui` — terminal-native live view using `ratatui`. Shows current phase, active goal, recent tool calls, queue, heartbeat, token usage. For headless installs, RPi setups, and SSH sessions.

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

---

## Install Experience

```bash
curl -sSf https://install.praxis.dev | bash
```

The install script:
1. Checks dependencies (Rust toolchain, Docker optional)
2. Prompts for Claude OAuth token and GitHub token
3. Opens a Claude-powered interview in the terminal (Haiku — fast, cheap, runs once)
4. Claude asks 6-8 natural conversational questions covering: who you are, what hardware you're running, what you spend too much time on, which messaging platform(s), timezone, security level preference, dashboard visibility, voice transcription
   It also asks for hard boundaries and "never do this" rules during setup so Praxis starts with at least a minimal boundary memory.
5. Claude synthesizes the conversation and writes initial files as a JSON blob
6. Script writes `IDENTITY.md`, `GOALS.md`, `ROADMAP.md`, `JOURNAL.md` (day 0 entry), `praxis.toml`
7. Clones the framework repo, runs `cargo build`, sets up cron via watchdog, fires first session

The installer should also support:
- `--dry-run` to show exactly which files, services, cron jobs, Docker assets, and credentials it will touch
- a zero-config home-server mode that chooses sane defaults for Raspberry Pi or Docker-first installs and prints how to change them later

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

[dependencies]
yoagent = "0.7"
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

### Completed

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

### Adopt Soon

- **Portable state export/import** — add `praxis export-memory`, `praxis import-memory`, and versioned backup/restore tooling for memories, preferences, identity files, and analytics so long-lived instances are durable across reinstalls or machine moves.
- **Schema migration policy** — every exported artifact and SQLite schema needs an explicit version and migration path.
- **Agent-core dependency hedge** — `yoagent` or any external agent-runtime dependency must sit behind a Praxis-owned abstraction with docs explaining its responsibilities, replacement plan, and exit strategy if the crate is abandoned.
- **Model transition controls** — add `model_pin`, model canaries, regression gates, and a "freeze on known-good model behavior" option so provider-side model updates do not silently change Praxis personality or reliability.
- **Boundary maintenance loop** — support `/boundaries` as a recurring conversation, and have the weekly alignment review explicitly ask whether hard limits changed.
- **Command semantics** — keep `/ask` synchronous and low-latency, `/run` asynchronous and session-based, and document that distinction everywhere the commands appear.
- **Attachment policy** — define explicit reject/chunk/summarize behavior for oversized files and never silently truncate.
- **Cold-memory decay clarification** — decay cold memories in place by default; only demote when certainty genuinely drops.
- **Audit exports** — add `/export-audit [days]` for human-readable reports of tool calls, memory writes, decisions, and file diffs.
- **File-mutation circuit breaker** — trip a guard if a session attempts to modify too much of the workspace, identity surface, or too many protected files at once.
- **Backup and restore** — optional automatic daily snapshots handled by Praxis or the watchdog.
- **Capability benchmarking** — add recurring capability tests and operator-specific replay/eval sessions to measure usefulness over time.
- **Speculative execution** — compare multiple rehearsed branches before committing to the safest or highest-yield act plan.
- **Wave execution** — group dependency-aware sub-agent work into parallel waves instead of spawning parallelism ad hoc.
- **Context-rot prevention** — make "fit work into clean context windows" a structural rule, not just a good habit.
- **Model profiles** — named execution modes like `quality`, `budget`, or `offline` that change provider/model behavior consistently across subsystems.
- **Append-only operational learnings** — sharpen `LEARNINGS.md` or an equivalent file into an append-only record of codebase-specific discoveries and instance conventions.
- **AGENTS-style pattern capture** — maintain a clearer "future runs need to know this" surface for project-specific conventions, gotchas, and workflow norms.
- **Wake-on-intent** — support approved interrupt-style wakes alongside scheduled sessions.
- **Reviewer cost guardrails** — keep reviewer context and token ceilings explicit so mandatory review stays affordable.
- **Watchdog heartbeat backstop** — pair the internal watchdog with a simple external liveness check.
- **Hierarchical goals** — support parent/child goals and dependency-aware progression once the richer lifecycle is stable.

### Future / Optional

- **Relational or graph memory layer** — add typed links between memories such as `caused_by`, `related_to`, `contradicts`, and `user_preference` without necessarily requiring a heavy graph database.
- **Hybrid semantic retrieval** — optionally layer vectors or semantic search on top of FTS5 once the keyword + preference graph approach proves insufficient.
- **Memory typing** — extend memory with episodic, semantic, and procedural classes, each with different decay and reinforcement rules.
- **Operator reinforcement commands** — allow `/reinforce <memory_id>` or message reactions to strengthen memories directly.
- **Conflict workbench** — create `MEMORY_CONFLICTS.md` or an equivalent workflow to surface conflicting memories with evidence and proposed resolution.
- **Adaptive context allocation** — learn which context sources correlate with successful outcomes for this specific operator and tune budgets accordingly.
- **Hierarchical summarization fallback** — aggressive summarization that preserves anchors like dates, goal IDs, boundaries, and names when context is tight.
- **Persistent context cache** — store a compressed working set from recent sessions for faster warm starts on constrained hardware.
- **Automatic anatomy refresh daemon** — beyond on-demand updates, optionally re-index changed files during idle windows.
- **Tool auto-documentation** — have Praxis maintain examples, failure modes, and reliability notes for installed tools and capabilities.
- **Cross-tool loop detection** — extend the loop guard to catch repeating multi-tool patterns, not just identical single-tool calls.
- **Community tool registry improvements** — include compatibility metadata, usage examples, and read-only community discovery.
- **Local multimodal processing** — optional on-device transcription, captioning, or light image understanding for privacy-preserving installs.
- **System anomaly correlation** — track CPU, memory, disk, and load anomalies against reviewer failures and bad outcomes.
- **Granular cooldown policies** — different approval windows per file or identity surface, potentially escalating some files to always-explicit approval.
- **Meta-evolution workflow** — let Praxis propose changes to the framework itself via `SELF_EVOLUTION.md`, with heavy approval gating.
- **Irreplaceability score** — track anticipation, follow-through, reliability, and operator dependence as a private metric, not as a vanity metric.
- **Adaptive scheduling** — let wake times and non-urgent session timing learn from actual operator behavior and quiet-hour patterns.
- **Local-first model fallback** — optionally use local or Ollama-compatible models for low-risk phases such as reflection or summarization.
- **Cargo feature modularity** — keep the single binary lean while allowing optional compile-time extras like voice, vector memory, or advanced graph features.
- **Auto-maintained docs** — let Praxis keep public docs and examples current as capabilities mature.
- **End-to-end replay testing** — run recorded transcript replays to catch behavioral regressions that unit tests miss.
- **Lite mode** — reduce sub-agent usage, tighten budgets, and simplify behavior for Raspberry Pi or low-power installs.
- **Energy budget / rate-limit budget** — model provider quota and operator attention as explicit resources that sessions consume.
- **Anonymous learning exchange** — possibly allow instances to publish sanitized, non-personal learnings to a shared registry later, but only with strong privacy guarantees.
- **WASM tool runtime** — support ultra-sandboxed community tools without granting broad local execution.
- **OpenTelemetry / Prometheus export** — richer external observability once local analytics and SSE are stable.
- **Local multimodal and local model bundles** — optional heavy extras for privacy-first or travel/offline deployments.
- **Postmortem generator** — automatically write structured failure postmortems after review or eval regressions.
- **Synthetic example generation** — turn high-value learnings into reusable structured examples for future prompt shaping or evaluation.
- **Social runtime** — optional scheduled outward-facing posting or status sharing on behalf of the operator.
- **VS Code ops surface** — lightweight editor integration for status, current goal, and safe run triggers.
- **PRD/story-mode dev runtime** — an optional developer-focused operating mode that works from explicit story state and stop signals.

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
