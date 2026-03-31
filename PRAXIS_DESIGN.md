# Praxis

**A standalone, self-evolving personal AI agent framework written in Rust.**

Praxis is designed for anyone to run their own instance of a personal AI agent that learns from them, grows with them, and becomes genuinely irreplaceable over time. It is not a chatbot. It is not a tool. It is an agent that wakes up on its own, pursues goals, builds capabilities, and shapes itself around the specific person running it.

The reference implementation is **Axonix** — a live instance running at [axonix.live](https://axonix.live), built and maintained by the framework's author. Everything in Praxis has been proven in Axonix first.

---

## Origin

Praxis synthesizes the best ideas from three projects:

- **Axonix** (`github.com/coe0718/axonix`) — the reference implementation. 18+ days of live evolution, 86+ sessions, 100% commit rate, 23,000+ lines written autonomously. Proven growth model, memory system, Telegram integration, dashboard, morning brief, prediction tracking, self-assessment. Written in Rust.
- **ClaudeClaw** (`github.com/moazbuilds/claudeclaw`) — a Claude Code plugin that proved people want always-on personal agents. Contributed: heartbeat pattern, quiet hours, security levels, voice/image/file attachments, Discord integration, model fallback.
- **earlyaidopters/claudeclaw** — a fork that contributed the 5-layer memory architecture concept, which Praxis improves upon significantly.

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
| Auth (webhook HMAC) | `hmac` + `sha1` |
| Config | `toml` (praxis.toml) |

Single binary distribution. Docker support via `docker-compose.yml`. Runs on Linux, macOS, Raspberry Pi.

---

## The Agent Loop

Every Praxis session runs through five phases in order. No phase assumes Git, no phase assumes a developer context. The loop is abstract — what happens in each phase depends on the instance's configuration and tools.

```
wake → orient → decide → act → reflect → sleep
```

### Orient
Load context according to the budget system (see below). Read identity, active goals, recent memory, predictions, and journal. Query the analytics table for relevant patterns. Receive any pending messages from Telegram/Discord. Know what today looks like before deciding anything.

### Decide
Choose what to work on. Source priority:
1. Incoming user messages (highest — operator spoke)
2. Urgent predictions expiring soon
3. Active goals from GOALS.md
4. Community input (GitHub issues with `agent-input` label, if enabled)
5. Self-directed goals from self-assessment

### Act
Do the work. This is the only phase that touches the outside world. It may involve:
- Writing and testing code (if a developer instance)
- Calling tools (HTTP, shell, structured)
- Spawning sub-agents for heavy tasks
- Sending messages via Telegram/Discord
- Updating files
- Posting to social (if enabled)

Act writes a session state file at each significant checkpoint so it can resume after a crash.

### Reflect
Capture memory from the session. Update GOALS.md, JOURNAL.md, METRICS.md, PATTERNS.md. Write analytics row to SQLite. Check identity files for proposed changes (see identity policy). Send session summary via Telegram/Discord if configured.

### Sleep
Wait for the next scheduled wake time. Respect quiet hours. Hand control to the watchdog.

---

## Memory System

Memory is what makes an instance feel like it knows you. It is the most important system in Praxis and the one most previous projects have gotten wrong.

### The problem with existing approaches
The earlyaidopters 5-layer system calls a model after every conversation turn to evaluate importance — that's an extra API call per message, adding latency and cost that compounds fast. Axonix's G-088 TF-IDF search is just a search layer with no capture pipeline feeding it. Neither system has forgetting.

### Praxis memory design

**Two storage tiers:**

`hot` — recent memories, full text, high specificity, FTS5 indexed. TTL of 30 days without access or reinforcement.

`cold` — consolidated insights, shorter, more abstract, weighted higher in search. Synthesized from clusters of hot memories. TTL of 90 days without reinforcement before demotion back to hot.

**Capture happens at session end, not per turn.** After the session completes, one single extraction pass runs over the full session transcript. One API call, full context, far better signal about what actually mattered. Things mentioned once in passing score low. Things returned to repeatedly score high.

**Daily consolidation.** Once per day (not on overflow), a consolidation pass looks for clusters of related hot memories and synthesizes them into cold memories. Predictable schedule, not reactive lumps.

**Reinforcement.** When a new hot memory connects to an existing cold one, the cold memory's weight increments rather than storing a duplicate. Memories that matter keep getting stronger.

**Contradiction detection.** When a new memory directly conflicts with an existing cold one, both are flagged. The conflict surfaces at the next session start so the agent can resolve it explicitly.

**Forgetting.** Hot memories expire after 30 days without access. Cold memories that haven't been reinforced in 90 days demote to hot before eventual expiry. The agent stays sharp rather than getting buried in its own history.

**Schema:**

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

**Rust modules:**
- `memory/capture.rs` — post-session extraction, importance scoring
- `memory/observer.rs` — transcript parsing, hot memory writes
- `memory/consolidator.rs` — daily consolidation pass
- `memory/search.rs` — unified FTS5 search across hot + cold
- `memory/loader.rs` — session startup injection within context budget

---

## Context Budget System

As instances grow older they accumulate memory, goals, journal entries, and observations. Without a budget system, the session prompt grows unboundedly until it breaks.

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
  { source = "journal",      priority = 6, max_pct = 0.10 },
  { source = "tools",        priority = 7, max_pct = 0.05 },
  { source = "task",         priority = 8, max_pct = 0.30 },
]
```

Each source fills in priority order. Sources exceeding their allocation get summarized via a single Haiku call. If the total still overflows, lower-priority sources are dropped entirely — never truncated mid-sentence. The agent is told explicitly what was excluded.

The agent can request additional context mid-session via a structured tool call. The budget system tracks actual usage over time and auto-tunes allocations — if identity consistently uses 2% of its 5% slot, the system reclaims the difference for memory.

---

## Tools System

Tools are declared in `praxis.toml` and discovered at startup. Three types:

**HTTP tools** — call any external API:
```toml
[[tool]]
name = "weather"
description = "Get current weather for a location"
type = "http"
method = "GET"
endpoint = "https://wttr.in/{location}?format=j1"
security_level = 1
```

**Shell tools** — run scripts on the host:
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

**Structured tools** — call internal Praxis functions:
```toml
[[tool]]
name = "memory_search"
description = "Search agent memory for relevant context"
type = "internal"
function = "memory::search"
security_level = 1
```

Security levels gate autonomous tool use:
- Level 1 — agent calls freely, read-only
- Level 2 — agent calls freely, write operations
- Level 3 — agent calls but logs every invocation
- Level 4 — requires user confirmation via Telegram/Discord before executing (60 second timeout)

The agent receives a tool manifest at session start and invokes tools via structured JSON output. Results are injected back into context automatically. Tool configs can be shared as gists or via a community registry — no Rust code required to extend Praxis.

---

## Multi-Agent / Sub-Agent Support

The primary agent can spawn sub-agents for tasks too large or too specialized for one context window.

**Orchestrator/worker model:**
- Primary agent has identity, memory, goals, full context
- Sub-agents are lightweight: task + relevant tools + memory slice + token budget
- Sub-agents execute and return structured output to primary
- **Sub-agents cannot write to memory or modify identity files**

**Three primary use cases:**

Research — a sub-agent searches the web and local memory in parallel, returns a structured brief, and exits. Primary doesn't burn context on the search.

Parallel execution — multiple sub-agents run simultaneously on independent subtasks via `tokio::join!`. Results merged by primary.

Specialized agents — a code reviewer sub-agent that fires when the primary produces a diff. A brief writer that fires at morning brief time. Each tuned with the right model, tools, and context slice.

Sub-agents default to Haiku (cheap, fast) unless the task requires reasoning, in which case the primary promotes them to Sonnet.

---

## Messaging

Three platforms, identical feature surface:

| Feature | Telegram | Discord | WhatsApp |
|---|---|---|---|
| Text commands | ✅ | ✅ | ✅ |
| Voice messages | ✅ | ✅ | ✅ |
| Images | ✅ | ✅ | ✅ |
| File attachments | ✅ | ✅ | ✅ |
| Push notifications | ✅ | ✅ | ✅ |
| Morning brief | ✅ | ✅ | ✅ |
| Support status | Official | Official | Unofficial (opt-in) |

**Commands available on all platforms:**
- `/ask <prompt>` — send a task or question
- `/run <task>` — spawn a mini-session for a specific job
- `/goal <description>` — add a goal to the backlog
- `/status` — current active goal + last commit/action
- `/brief` — on-demand morning brief
- `/health` — CPU, memory, disk, uptime

**File attachments** — text-readable documents (`.txt`, `.json`, `.csv`, `.log`, `.yaml`, etc.) are downloaded via the platform's file API, read as UTF-8, and injected into the prompt. Cap at 100KB with a friendly "file too large" reply if exceeded.

**Voice messages** — transcribed to text before being sent to Claude. Requires a Whisper-compatible transcription endpoint (configurable, optional).

**WhatsApp** is unofficial (uses `whatsapp-web.js` style automation), opt-in only, and carries a risk disclosure in the install script. Not a selling point — a courtesy for users who live in WhatsApp.

---

## Quiet Hours

```toml
[schedule]
cron = "0 */4 * * *"
quiet_hours_start = "23:00"
quiet_hours_end = "07:00"
timezone = "America/Chicago"
```

Sessions scheduled during quiet hours are deferred to the first window after quiet hours end. Telegram/Discord messages received during quiet hours are queued and delivered at the start of the next session. The morning brief fires at the first session after quiet hours end.

---

## Security Levels

Four levels controlling what the agent can do autonomously:

- **Level 1 (Observe)** — read files, search memory, call read-only APIs
- **Level 2 (Suggest)** — write to its own files, send messages, call write APIs
- **Level 3 (Act)** — run shell commands, modify project files, deploy
- **Level 4 (Full)** — unrestricted system access, requires explicit operator consent

Default is Level 2. Level 3 and 4 require explicit opt-in during install. Individual tools can require a higher level than the instance default.

---

## Identity Policy

Three tiers of files with different write permissions:

**Freely writable by agent:**
`GOALS.md`, `JOURNAL.md`, `METRICS.md`, `LEARNINGS.md`, `PATTERNS.md`, `predictions.json`, `praxis.db`, `CAPABILITIES.md`, `DAY_COUNT`

**Cooldown-gated (24hr approval window):**
`IDENTITY.md`, `ROADMAP.md`

Agent proposes changes. Operator receives a diff via Telegram/Discord: *"Your agent wants to update IDENTITY.md. Reply /approve or /reject within 24 hours. No response = approved."* This preserves organic evolution while giving operators a chance to catch drift they don't want.

**Locked (operator only):**
`praxis.toml`, `.env`, tool definitions, security config

Agent cannot modify these under any circumstances. If the agent wants a new tool or different security level, it writes a structured proposal to `PROPOSALS.md` for the operator to action.

---

## Growth Model

Every Praxis instance works through five levels. The levels are universal — the content of each level is personal to the specific instance and operator.

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

**For humans:** Live dashboard at a configurable URL showing active goals, predictions, journal, metrics, session stream. Built and maintained by the agent itself using `build_site.py` (Python script run as a subprocess at session end).

**For the agent:** Structured analytics table in SQLite the agent can actually query and reason over:

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
  phase_durations TEXT,  -- JSON: {"orient": 12, "decide": 3, "act": 847, "reflect": 24}
  stuck_signals INTEGER,
  outcome TEXT           -- "success" | "partial" | "blocked" | "error"
);
```

Analytical findings persist in `PATTERNS.md` — not journal entries but durable conclusions the agent injects at session start as high-priority context. "Sessions starting after 6pm have 40% lower completion rates." "Goals involving external APIs take 2.3x longer."

---

## Auto-Update System

Two-process architecture:

**`praxis-watchdog`** — tiny, stable process. Owns the cron schedule, monitors the main process, handles updates. Downloads new binaries, verifies hash, swaps during the sleep window between sessions (never mid-session), restarts. Almost never needs updating itself.

**`praxis`** — the main binary. Can be safely updated because watchdog manages the swap.

```toml
[updates]
channel = "stable"        # stable | beta | pin
auto_update = true
notify_before = true      # Telegram/Discord message before updating
rollback_on_failure = true
```

If a post-update session fails, watchdog automatically rolls back to the previous binary and notifies the operator. Previous binary is always kept for one version.

---

## Privacy

Every external surface defaults to off or private:

```toml
[dashboard]
enabled = true
bind = "127.0.0.1"    # localhost only by default
port = 8080
auth = true
auth_token = ""       # generated at install time

[dashboard.public]
enabled = false       # explicit opt-in required
domain = ""

[social]
bluesky = false
github_discussions = false

[community]
public_issues = false  # only operator can file issues by default
```

Auth tokens are generated at install time using `rand` + `base64`, stored in `.env`, rotatable via `/rotate-token`.

---

## Install Experience

```bash
curl -sSf https://install.praxis.dev | bash
```

The install script:
1. Checks dependencies (Rust toolchain, Docker optional)
2. Prompts for Claude OAuth token and GitHub token
3. Opens a Claude-powered interview **in the terminal** using the user's own token
4. Claude asks 6-8 natural conversational questions:
   - Who are you and what do you do?
   - What hardware is this running on?
   - What do you spend time on that you wish you didn't?
   - What would this agent do first to be useful?
   - What's your daily rhythm / timezone?
   - Which messaging platform? (Telegram / Discord / WhatsApp / multiple)
   - How much do you trust your agent? (sets security level)
   - Do you want a public dashboard?
5. Claude synthesizes the conversation and writes initial files as a JSON blob
6. Script writes `IDENTITY.md`, `GOALS.md`, `ROADMAP.md`, `JOURNAL.md` (day 0 entry), `praxis.toml`
7. Clones the framework repo, runs `cargo build`, sets up cron via watchdog, fires first session

The interview uses **Haiku** — fast, cheap, runs once. The agent's first session starts with files that feel personal because they came from a real conversation.

---

## Repository Structure

```
praxis/
├── src/
│   ├── main.rs              # Entry point, CLI args
│   ├── loop/
│   │   ├── mod.rs
│   │   ├── orient.rs        # Context loading, budget allocation
│   │   ├── decide.rs        # Goal selection, message queue
│   │   ├── act.rs           # Tool invocation, sub-agent spawning
│   │   ├── reflect.rs       # Memory capture, file updates, analytics
│   │   └── session.rs       # Session state, crash recovery
│   ├── memory/
│   │   ├── mod.rs
│   │   ├── capture.rs       # Post-session extraction
│   │   ├── consolidator.rs  # Daily consolidation pass
│   │   ├── search.rs        # Unified FTS5 search
│   │   └── loader.rs        # Session startup injection
│   ├── messaging/
│   │   ├── mod.rs
│   │   ├── telegram.rs      # Telegram Bot API
│   │   ├── discord.rs       # Discord bot
│   │   ├── whatsapp.rs      # Unofficial WhatsApp (opt-in)
│   │   └── attachments.rs   # Voice, image, file handling
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── registry.rs      # Tool discovery from praxis.toml
│   │   ├── http.rs          # HTTP tool executor
│   │   ├── shell.rs         # Shell tool executor
│   │   └── security.rs      # Security level gating
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── primary.rs       # Primary agent with full context
│   │   └── subagent.rs      # Lightweight worker agents
│   ├── context/
│   │   ├── mod.rs
│   │   └── budget.rs        # Token budget, priority queue, auto-tune
│   ├── identity/
│   │   ├── mod.rs
│   │   └── policy.rs        # File tier enforcement, cooldown gating
│   ├── dashboard/
│   │   ├── mod.rs
│   │   └── server.rs        # axum server, auth, static files
│   ├── watchdog/
│   │   ├── mod.rs
│   │   └── updater.rs       # Binary swap, rollback
│   ├── scheduler/
│   │   ├── mod.rs
│   │   └── quiet_hours.rs   # Cron + quiet hour gating
│   └── analytics/
│       ├── mod.rs
│       └── patterns.rs      # Session analytics, PATTERNS.md
├── skills/                  # Reusable prompt templates
├── scripts/
│   ├── install.sh           # curl-pipe installer
│   ├── evolve.sh            # Trigger a full session
│   └── interview.sh         # Claude-powered install interview
├── docs/                    # Dashboard static files (agent-maintained)
├── caddy/                   # Reverse proxy config (optional)
├── .env.example
├── praxis.toml.example      # Annotated config template
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml
├── IDENTITY.md              # Agent-maintained, cooldown-gated
├── GOALS.md                 # Agent-maintained, freely writable
├── ROADMAP.md               # Agent-maintained, cooldown-gated
├── JOURNAL.md               # Agent-maintained, freely writable
├── METRICS.md               # Agent-maintained, freely writable
├── PATTERNS.md              # Agent-maintained, freely writable
├── LEARNINGS.md             # Agent-maintained, freely writable
├── CAPABILITIES.md          # Agent-maintained, freely writable
├── PROPOSALS.md             # Agent proposes config changes here
└── DAY_COUNT                # Incrementing day counter
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

## Relationship to Axonix

Axonix is the reference implementation of Praxis. It runs on dedicated hardware, evolves every 4 hours, and proves out every feature before it ships in the framework. When Axonix builds something new and it works well, that pattern gets extracted into Praxis. When Praxis adds something new, Axonix picks it up in its next session.

Axonix is to Praxis what a founder's own usage is to a product — the sharpest feedback loop possible, because the person building it is also the primary user.

---

## What Praxis Is Not

- Not a hosted service. You run it yourself.
- Not a chatbot. It has its own goals and wakes up without being asked.
- Not a coding agent specifically. Developers will use it for coding. Writers will use it for writing. Sysadmins will use it for monitoring. The loop is abstract.
- Not finished. The framework will evolve alongside Axonix. Early adopters should expect rough edges and contribute back.

---

*Praxis. Theory becoming action.*
