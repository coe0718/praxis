# Praxis

**A standalone, self-evolving personal AI agent framework written in Rust.**

Praxis is designed for anyone to run their own instance of a personal AI agent that learns from them, grows with them, and becomes genuinely irreplaceable over time. It is not a chatbot. It is not a tool. It is an agent that wakes up on its own, pursues goals, builds capabilities, and shapes itself around the specific person running it.

The reference implementation is **Axonix** — a live instance running at [axonix.live](https://axonix.live), built and maintained by the framework's author. Everything in Praxis has been proven in Axonix first.

---

## Origin

Praxis synthesizes the best ideas from several projects:

- **Axonix** (`github.com/coe0718/axonix`) — the reference implementation. 18+ days of live evolution, 86+ sessions, 100% commit rate, 23,000+ lines written autonomously. Proven growth model, memory system, Telegram integration, dashboard, morning brief, prediction tracking, self-assessment. Written in Rust.
- **ClaudeClaw** (`github.com/moazbuilds/claudeclaw`) — proved people want always-on personal agents. Contributed: heartbeat pattern, quiet hours, security levels, voice/image/file attachments, Discord integration, model fallback.
- **earlyaidopters/claudeclaw** — contributed the 5-layer memory architecture concept, which Praxis improves upon significantly.
- **secure-openclaw** (ComposioHQ) — contributed: sender pairing code security model, Signal platform support, multi-provider architecture.
- **claude-caliper** (nikhilsitaram) — contributed: no self-review rule, success criteria JSON per goal, Brier score calibration.
- **superpowers** (obra) — contributed: skill dependency chaining and composable skill resolver.
- **get-shit-done** (gsd-build) — contributed: context handoff notes, spec-driven development pattern.
- **tinyagi** (TinyAGI) — contributed: SSE event streaming architecture, TUI dashboard concept.
- **openfang** (RightNow-AI) — contributed: loop guard / circuit breaker pattern, signed tool manifests.
- **claude-code-agents** (undeadlist) — contributed: parallel sub-agent audit patterns.

---

## Philosophy

**One instance per person.** Praxis is not a shared service. You run it on your own hardware — a VPS, a Raspberry Pi, a home server. Your data stays yours. Your agent shapes itself around you specifically, not around a generic user profile.

**No API billing surprises.** Praxis uses a Claude Pro OAuth token (`sk-ant-oat01-...`), not the Anthropic API. It runs against your existing Pro subscription with no per-token charges.

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
| Agent core | `yoagent` |
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
Load context according to the budget system. Read identity, active goals, recent memory, predictions, and PATTERNS.md. Query the analytics table for relevant performance patterns. Receive any pending messages from messaging platforms. Know what today looks like before deciding anything.

### Decide
Choose what to work on. Source priority:
1. Incoming user messages (highest — operator spoke)
2. Urgent predictions expiring soon
3. Active goals from GOALS.md
4. Community input (GitHub issues with `agent-input` label, if enabled)
5. Self-directed goals from self-assessment

For any Level 3+ goal, Decide produces a structured plan including assumptions, risks, and explicit success criteria before proceeding to Act. The loop pauses for operator confirmation if the goal's security level requires it.

### Act
Do the work. This is the only phase that touches the outside world. It may involve writing code, calling tools, spawning sub-agents, sending messages, updating files, or posting to social. Act writes a session state checkpoint at each significant step so it can resume after a crash.

The **loop guard** runs throughout Act: a SHA256 hash of recent tool invocations is maintained in the session state file. If the same tool+args hash appears 3+ times consecutively, Act breaks immediately, notifies the operator via messaging, and waits. This prevents runaway tool loops from consuming tokens indefinitely.

### Reflect
After Act completes, Reflect runs **before** marking a goal done:
1. Spawn a reviewer sub-agent with a fresh minimal context window
2. Reviewer checks the work against the goal's success criteria JSON (if it exists)
3. If criteria pass → mark complete, capture memory, update files
4. If criteria fail → return goal to active with reviewer's findings attached

Memory capture runs at session end (not per-turn). One extraction pass over the full session transcript. Files updated: GOALS.md, JOURNAL.md, METRICS.md, PATTERNS.md. Analytics row written to SQLite.

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

**Forgetting.** Hot memories expire after 30 days without access. Cold memories demote to hot after 90 days without reinforcement before eventual expiry.

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
```

### Rust modules
- `memory/capture.rs` — post-session extraction, importance scoring
- `memory/consolidator.rs` — daily consolidation, contradiction detection
- `memory/search.rs` — unified FTS5 search across hot + cold
- `memory/loader.rs` — session startup injection within context budget
- `memory/decay.rs` — TTL enforcement, demotion logic

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

**File attachments** — text-readable documents downloaded via the platform's file API, read as UTF-8, injected into the prompt. Cap at 100KB with a friendly reply if exceeded.

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

---

## Security Levels

- **Level 1 (Observe)** — read files, search memory, call read-only APIs
- **Level 2 (Suggest)** — write to own files, send messages, call write APIs
- **Level 3 (Act)** — run shell commands, modify project files, deploy
- **Level 4 (Full)** — unrestricted system access, explicit operator consent required

Default is Level 2. Levels 3 and 4 require explicit opt-in during install.

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
agent:memory_capture {"importance": 0.8}
agent:reviewer_spawned {"goal_id": "G-096"}
agent:goal_complete {"goal_id": "G-096"}
agent:reflect_complete
```

Any frontend subscribes to this stream. The dashboard is just one consumer. New interfaces can be built without touching the agent core.

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
5. Claude synthesizes the conversation and writes initial files as a JSON blob
6. Script writes `IDENTITY.md`, `GOALS.md`, `ROADMAP.md`, `JOURNAL.md` (day 0 entry), `praxis.toml`
7. Clones the framework repo, runs `cargo build`, sets up cron via watchdog, fires first session

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
│   │   └── decay.rs
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
│   │   └── budget.rs          # Token budget, priority queue, handoff notes
│   ├── identity/
│   │   └── policy.rs          # File tier enforcement, cooldown gating
│   ├── quality/
│   │   ├── reviewer.rs        # No self-review enforcement
│   │   ├── criteria.rs        # Success criteria JSON loader + runner
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
│       └── patterns.rs
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

1. **Agent loop skeleton** — orient/decide/act/reflect/sleep with phase state file
2. **Context budget system** — priority queue, auto-tune, handoff notes
3. **Memory system** — hot/cold SQLite FTS5, capture, consolidation, decay
4. **Tools system** — HTTP, shell, loop guard, security levels, approval queue
5. **Identity policy** — file tier enforcement, cooldown gating
6. **SSE event streaming** — shapes the dashboard architecture
7. **Messaging layer** — Telegram first, then Discord, Signal, WhatsApp
8. **Quality system** — success criteria JSON, no self-review rule, Brier scores
9. **Analytics + observability** — sessions table, PATTERNS.md, web dashboard, TUI
10. **Watchdog + auto-update** — add before first public release

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
