# Praxis

Self-hosted personal AI agent infrastructure in Rust.

Praxis is built for one operator, long-running local state, and safe autonomous behavior. It is not a hosted assistant and not a generic chatbot shell. The goal is a private, durable agent runtime that can wake up, inspect its own world, learn from execution history, and become more useful over time without losing operator trust.

The reference implementation is Axonix. Praxis is the framework extraction of those patterns into a reusable codebase.

## Why Praxis

- Private by default: local files, local SQLite, self-hosted runtime.
- Built to last: identity, goals, memory, analytics, and proposals survive across sessions.
- Safety-first autonomy: approvals, path policy checks, loop guards, quality gates, and deterministic offline fallbacks.
- Rust and Docker friendly: single binary, small modules, and container-ready workflows from the start.

## Current Status

Praxis is in active development, but it is already a working local operator runtime.

Shipped today:

- Lightweight `praxis ask ...` prompts that do not create or mutate session state
- Resumable `orient -> decide -> act -> reflect -> sleep` session loop
- Linux/macOS path handling plus Docker-first data-dir support
- Markdown identity and goal files with dependency-aware goal selection
- SQLite-backed sessions, approvals, memories, reviews, evals, forensics, learning runs, and provider usage
- Context budgeting, anatomy indexing, repeated-read avoidance, and token/cost ledgers
- Anchor-preserving summarization for oversized context sources
- Adaptive context allocation that nudges source budgets based on prior successful sessions
- Hot/cold memory search plus operational memory for do-not-repeat notes and known bugs
- Tool registry, approval queue, path policy enforcement, and loop guards for identical or repeating multi-step tool patterns
- First real tool execution path for approved append-only writes inside allowed Praxis data files
- Auto-maintained `CAPABILITIES.md` notes for installed tools, recent examples, and failure history
- Provider routing with `stub`, Claude, OpenAI, Ollama, and router-mode failover
- Explicit `budgets.toml` limits for ask/run attempt count, token spend, and estimated cost
- Opt-in local-first fallback so low-risk `ask` and `act` phases can prefer Ollama before cloud providers
- Runtime heartbeat file plus `praxis heartbeat check` and `scripts/check-heartbeat.sh` for external liveness checks
- Telegram operator commands and a lightweight SSE/dashboard server
- Reviewer/eval quality gates during Reflect
- Automatic markdown postmortems for reviewer failures, eval regressions, and similar bad outcomes
- Fixture-backed replay tests for stateful session, forensics, and approval flows
- Argus analysis for drift, repeated work, failures, and token hotspots
- Learning runtime that mines opportunities and syncs them into `PROPOSALS.md`
- Opportunity acceptance that promotes mined work into durable goals in `GOALS.md`
- Manifest-versioned state export/import plus human-readable audit export
- `AGENTS.md` pattern capture with CLI support for future-run conventions and gotchas

Not finished yet:

- Broader tool execution beyond the first controlled data-write path
- Watchdog heartbeat, rollout canaries, and rollback automation
- Full watchdog/auto-update process management beyond the new heartbeat backstop
- Richer dashboard UI and additional messaging platforms
- Automatic scheduled backup snapshots for long-lived state
- Deeper memory consolidation, reinforcement, and longer-horizon calibration

## Quick Start

Initialize a local Praxis data directory:

```bash
cargo run -- --data-dir ./local-data init --name "Praxis" --timezone UTC
```

Run a single session:

```bash
cargo run -- --data-dir ./local-data run --once
```

Inspect the current state:

```bash
cargo run -- --data-dir ./local-data status
```

Validate the installation:

```bash
cargo run -- --data-dir ./local-data doctor
```

By default Praxis uses the deterministic `stub` backend, so the basic workflow works fully offline.

## Provider Setup

`praxis init` seeds both `praxis.toml` and `providers.toml`.

Use a single remote backend:

```toml
[agent]
backend = "claude"
```

```bash
export ANTHROPIC_API_KEY=...
```

Or enable routed failover:

```toml
[agent]
backend = "router"
```

```toml
[[providers]]
provider = "claude"
model = "claude-3-5-sonnet-latest"

[[providers]]
provider = "openai"
model = "gpt-5.4-mini"

[[providers]]
provider = "ollama"
model = "llama3.2"
base_url = "http://127.0.0.1:11434"
```

When router mode is active, Praxis records every provider attempt, token count, and estimated cost in SQLite so `status` and Argus can explain what actually happened.

## Docker

Praxis is intended to stay runnable in Docker from the start.

Initialize container state:

```bash
docker compose run --rm praxis-init
```

Run one session:

```bash
docker compose run --rm praxis-run
```

Check status:

```bash
docker compose run --rm praxis-status
```

The compose file binds `./docker-data` to `/var/lib/praxis`, so state persists across container runs.

## Core Workflows

### Daily Runtime

- `praxis ask ...`
- `praxis run --once`
- `praxis status`
- `praxis doctor`
- `praxis forensics latest`
- `praxis argus --limit 10`

`praxis ask ...` is synchronous and stateless. `praxis run --once` is a real session run that updates durable Praxis state.

### Approvals and Tools

- `praxis queue`
- `praxis approve <id>`
- `praxis reject <id>`
- `praxis tools list`
- `praxis tools register ...`
- `praxis tools request ...`

Current real execution slice:

```bash
cargo run -- --data-dir ./local-data tools request \
  --name praxis-data-write \
  --summary "Append reviewed journal note" \
  --write-path JOURNAL.md \
  --append-text "Operator approved this note."
```

After approval, `praxis run --once` will execute that request by appending the approved text to the declared allowed file inside the Praxis data directory.

### Learning and Opportunity Mining

Praxis can ingest notes from `learning/sources/`, synthesize learnings, detect repeated work, and create a throttled proposal queue:

```bash
cargo run -- --data-dir ./local-data learn note "Prefer cargo test --locked before pushing"
cargo run -- --data-dir ./local-data learn run
cargo run -- --data-dir ./local-data learn list
cargo run -- --data-dir ./local-data learn accept 1
cargo run -- --data-dir ./local-data learn dismiss 2
```

`LEARNINGS.md` is now an append-only operational log. Manual `learn note` entries and automatic source syntheses both append structured timestamped records instead of rewriting history.

Accepted opportunities are not just status changes. Praxis links them into `PROPOSALS.md` and promotes them into `GOALS.md`, so the main loop can pick them up as real work later.

### Future-Run Notes

Praxis now has an explicit `AGENTS.md` surface for project-specific conventions, gotchas, and handoff notes that future sessions should load early:

```bash
cargo run -- --data-dir ./local-data agents view
cargo run -- --data-dir ./local-data agents add --section workflow --note "Prefer project-local scripts over one-off shell commands."
cargo run -- --data-dir ./local-data agents add --section gotcha --note "Docker rebuilds are expected to take a while after backend changes."
```

That file is part of the foundation set, is loaded into Orient as its own context source, and is included in portable exports/imports.

### Durability and Audit

Praxis can now export a portable state bundle, import it into a different data directory, and generate a human-readable audit report:

```bash
cargo run -- --data-dir ./local-data export state --output ./praxis-backup
cargo run -- --data-dir ./restored-data import --input ./praxis-backup
cargo run -- --data-dir ./local-data export audit --output ./audit.md --days 30
```

State bundles include a versioned manifest, the SQLite schema version, runtime state, config, tools, goals, evals, learning sources, and the core markdown identity files. Imports re-home the config to the target data directory so restores stay portable across machines and Docker paths.

### Messaging and Live Views

Telegram support currently includes doctoring, polling, and command routing:

```bash
export PRAXIS_TELEGRAM_BOT_TOKEN=...
export PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS=12345,67890

cargo run -- --data-dir ./local-data telegram doctor
cargo run -- --data-dir ./local-data telegram poll-once
cargo run -- --data-dir ./local-data telegram run --cycles 0
```

Messaging semantics match the CLI split:

- `/ask <prompt>` is low-latency and does not create a Praxis session.
- `/run <task>` executes a real stateful session and bypasses quiet-hours deferral because the operator explicitly requested it.

Run the local dashboard/SSE server:

```bash
cargo run -- --data-dir ./local-data serve --host 127.0.0.1 --port 8787
```

## Architecture At A Glance

- `src/loop/`: session runtime and phase orchestration
- `src/context/`: budget engine and context assembly
- `src/anatomy.rs`: file summaries, token estimates, and repeated-read avoidance
- `src/identity/`: foundation files, goal parsing, and goal promotion
- `AGENTS.md`: future-run conventions, gotchas, and handoff notes
- `src/memory/`: memory loading plus operational memory support
- `src/storage/`: SQLite persistence for runtime state and analytics
- `src/tools/`: registry, policy, approval flow, and loop guards
- `src/backend/` and `src/providers/`: model/provider execution and routing
- `src/quality/`: reviewer and eval gates
- `src/learning/`: learning runtime, opportunity mining, and proposal sync
- `src/argus/`: drift, failure, repeated-work, and token hotspot analysis
- `src/messaging/` and `src/dashboard/`: operator surfaces
- `tests/`: end-to-end CLI coverage

The codebase follows a small-module style and aims to keep Rust source files under 250 lines.

## Development

Run the main verification steps locally:

```bash
cargo fmt --all
cargo test --locked
docker build --tag praxis:ci .
```

The project currently targets offline-deterministic tests by default. Live providers and messaging surfaces are wired behind explicit configuration and environment variables.

## Design And Roadmap

The canonical design document is [PRAXIS_DESIGN.md](PRAXIS_DESIGN.md). It tracks:

- core runtime architecture
- adopted ideas and future enhancements
- build-order status
- what is already implemented versus what is still planned

## License

MIT
