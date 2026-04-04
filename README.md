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

- Resumable `orient -> decide -> act -> reflect -> sleep` session loop
- Linux/macOS path handling plus Docker-first data-dir support
- Markdown identity and goal files with dependency-aware goal selection
- SQLite-backed sessions, approvals, memories, reviews, evals, forensics, learning runs, and provider usage
- Context budgeting, anatomy indexing, repeated-read avoidance, and token/cost ledgers
- Hot/cold memory search plus operational memory for do-not-repeat notes and known bugs
- Tool registry, approval queue, path policy enforcement, and loop-guard protection
- Provider routing with `stub`, Claude, OpenAI, Ollama, and router-mode failover
- Telegram operator commands and a lightweight SSE/dashboard server
- Reviewer/eval quality gates during Reflect
- Argus analysis for drift, repeated work, failures, and token hotspots
- Learning runtime that mines opportunities and syncs them into `PROPOSALS.md`
- Opportunity acceptance that promotes mined work into durable goals in `GOALS.md`

Not finished yet:

- Real tool execution beyond the current safe stub path
- Watchdog heartbeat, rollout canaries, and rollback automation
- Richer dashboard UI and additional messaging platforms
- Export/import and backup workflows for long-lived state
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

- `praxis run --once`
- `praxis status`
- `praxis doctor`
- `praxis forensics latest`
- `praxis argus --limit 10`

### Approvals and Tools

- `praxis queue`
- `praxis approve <id>`
- `praxis reject <id>`
- `praxis tools list`
- `praxis tools register ...`
- `praxis tools request ...`

### Learning and Opportunity Mining

Praxis can ingest notes from `learning/sources/`, synthesize learnings, detect repeated work, and create a throttled proposal queue:

```bash
cargo run -- --data-dir ./local-data learn run
cargo run -- --data-dir ./local-data learn list
cargo run -- --data-dir ./local-data learn accept 1
cargo run -- --data-dir ./local-data learn dismiss 2
```

Accepted opportunities are not just status changes. Praxis links them into `PROPOSALS.md` and promotes them into `GOALS.md`, so the main loop can pick them up as real work later.

### Messaging and Live Views

Telegram support currently includes doctoring, polling, and command routing:

```bash
export PRAXIS_TELEGRAM_BOT_TOKEN=...
export PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS=12345,67890

cargo run -- --data-dir ./local-data telegram doctor
cargo run -- --data-dir ./local-data telegram poll-once
cargo run -- --data-dir ./local-data telegram run --cycles 0
```

Run the local dashboard/SSE server:

```bash
cargo run -- --data-dir ./local-data serve --host 127.0.0.1 --port 8787
```

## Architecture At A Glance

- `src/loop/`: session runtime and phase orchestration
- `src/context/`: budget engine and context assembly
- `src/anatomy.rs`: file summaries, token estimates, and repeated-read avoidance
- `src/identity/`: foundation files, goal parsing, and goal promotion
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
