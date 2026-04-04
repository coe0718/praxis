# Praxis

Praxis is a standalone, self-evolving personal AI agent framework written in Rust.

It is designed to be self-hosted, private by default, and shaped around one operator instead of a generic user profile. The long-term goal is not to build another chatbot. The goal is to build an always-on personal agent that learns, develops capabilities, and becomes genuinely irreplaceable over time.

The reference implementation is Axonix. Praxis is the framework extraction of those ideas into a reusable codebase.

## Current status

Praxis has moved past the bare foundation milestone and now ships a working local operator runtime.

Implemented so far:

- Rust library-first crate with a single `praxis` binary
- TOML config loading and validation
- Cross-platform Linux/macOS data-dir handling
- Resumable `orient -> decide -> act -> reflect -> sleep` loop
- SQLite-backed sessions, approvals, memories, reviews, and eval runs
- Context budgeting plus hot/cold memory search with SQLite FTS
- Markdown identity, goals, criteria, and eval definition files
- Tool registry, approval queue, path policy checks, and loop-guard protection
- Dependency-aware goal selection with `blocked_by` and `wake_when` metadata
- A formal stop condition when all tracked goals are complete
- Config-driven provider routing with Claude, OpenAI, Ollama, and router-mode failover
- Telegram operator commands and a lightweight polling loop
- SSE/dashboard server with summary and recent-event views
- Deterministic reviewer and operator-eval quality gates during Reflect
- SQLite-backed phase snapshots plus CLI forensics replay
- Argus performance analysis for recent session quality trends
- Session-level provider attempt, token, and estimated cost tracking
- Deterministic offline tests plus Docker-first packaging

Not implemented yet:

- Real tool execution beyond the current safe stub path
- Memory consolidation, reinforcement, and decay workflows
- Drift detection and longer-horizon calibration reports
- Watchdog heartbeat, canary sessions, and rollback automation
- Richer dashboard/UI and more messaging surfaces

## Quick start

Initialize a local Praxis data directory:

```bash
cargo run -- --data-dir ./local-data init --name "Praxis" --timezone UTC
```

Run one session:

```bash
cargo run -- --data-dir ./local-data run --once
```

Inspect the current state:

```bash
cargo run -- --data-dir ./local-data status
```

Validate the setup:

```bash
cargo run -- --data-dir ./local-data doctor
```

By default Praxis uses the deterministic `stub` backend. `praxis init` also seeds a `providers.toml` file that defines the provider chain Praxis can use for direct or routed execution.

To enable a single remote backend, set `agent.backend = "claude"` or `agent.backend = "openai"` in `praxis.toml` and export the matching credential:

```bash
export ANTHROPIC_API_KEY=...
export OPENAI_API_KEY=...
```

To enable failover, set `agent.backend = "router"` in `praxis.toml`. Praxis will walk the ordered routes in `providers.toml` until one succeeds, and `praxis status` will report the latest provider, attempt count, failure count, token usage, and estimated cost.

## Operator commands

Core lifecycle:

- `praxis init`
- `praxis run --once`
- `praxis status`
- `praxis doctor`
- `praxis argus --limit 10`
- `praxis forensics latest`

Approvals and tool queue:

- `praxis queue`
- `praxis approve <id>`
- `praxis reject <id>`
- `praxis tools list`
- `praxis tools register ...`
- `praxis tools request ...`

Messaging and live views:

- `praxis telegram doctor`
- `praxis telegram poll-once`
- `praxis telegram run --cycles 0`
- `praxis telegram send --chat-id <id> --text "hello"`
- `praxis serve --host 127.0.0.1 --port 8787`

Telegram uses:

```bash
export PRAXIS_TELEGRAM_BOT_TOKEN=...
export PRAXIS_TELEGRAM_ALLOWED_CHAT_IDS=12345,67890
```

## Quality gates

Reflect now enforces local quality checks before finalizing the session outcome:

- goal-specific review criteria live in `goals/criteria/<goal-id>.json`
- operator evals live in `evals/*.json`
- review failures surface as `review_failed`
- eval regressions surface as `eval_failed`
- `praxis status` shows the latest review result and eval summary

The foundation data directory seeds one example goal criteria file and one smoke eval so the behavior is visible immediately after `init`.

## Goal metadata

Goals can now express lightweight dependency and trigger state directly in `GOALS.md`:

```md
- [ ] G-002: Ship the dependent feature
  blocked_by: G-001
  wake_when: env:PRAXIS_RELEASE_READY
```

Praxis will prefer prerequisite goals before blocked dependents, skip goals whose trigger is not active, and emit `stop_condition_met` once everything currently tracked is complete.

## Forensics and Argus

Praxis now records phase-boundary snapshots in SQLite during a session so you can replay what happened later:

```bash
cargo run -- --data-dir ./local-data forensics latest
```

Argus is a lightweight performance director that analyzes recent session failures and produces concrete improvement directives:

```bash
cargo run -- --data-dir ./local-data argus --limit 10
```

## Provider routing

`providers.toml` is separate from `praxis.toml` so the runtime backend choice stays simple while route details remain editable:

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

When router mode is active, Praxis tries each route in order for both the Decide and Act phases. Every attempt is stored in SQLite so status and future analytics can see which provider actually handled the session and what the estimated cost was.

## Docker

Praxis is meant to stay runnable in Docker from the start.

Initialize container data:

```bash
docker compose run --rm praxis-init
```

Run one session in Docker:

```bash
docker compose run --rm praxis-run
```

Check status:

```bash
docker compose run --rm praxis-status
```

The compose setup binds `./docker-data` to `/var/lib/praxis` in the container so state persists across runs.

You can also run validation and the dashboard from containers if you want the whole stack isolated:

```bash
docker compose run --rm praxis-status praxis doctor
docker compose run --rm --service-ports praxis-run praxis serve --host 0.0.0.0 --port 8787
```

## Project shape

The current codebase is organized around small modules with a preference for keeping Rust source files under 250 lines.

- `src/cli/`: CLI surface and command dispatch
- `src/config.rs`: typed config schema and validation
- `src/paths.rs`: data directory and path resolution
- `src/state.rs`: persisted session checkpoint state
- `src/context/`: budget engine and local context assembly
- `src/identity/`: markdown identity and goal parsing/policy
- `src/memory/`: memory types and loading logic
- `src/storage/`: SQLite persistence for sessions, approvals, memories, reviews, evals, and provider attempts
- `src/loop/`: runtime loop orchestration
- `src/quality/`: reviewer criteria and operator eval handling
- `src/messaging/`: Telegram operator bridge
- `src/dashboard/`: SSE/dashboard server
- `src/providers/`: provider route config and cost metadata
- `src/backend/`: provider execution and router failover
- `tests/`: end-to-end CLI coverage

## Design direction

The broader Praxis vision includes:

- hot and cold memory tiers
- context budgeting and handoff notes
- tool registry and approval queues
- messaging interfaces
- dashboard and SSE streaming
- reviewer/eval quality gates that grow toward true sub-agent review
- a watchdog-managed update path

The detailed design and long-form architecture notes live in [PRAXIS_DESIGN.md](./PRAXIS_DESIGN.md).

## Development

Run formatting and tests locally:

```bash
cargo fmt
cargo test --locked
docker build --tag praxis:ci .
```

GitHub Actions runs `cargo fmt --check`, `cargo test --locked`, `docker compose config`, and a Docker smoke build on pushes to `main` and on pull requests.

## Philosophy

- One instance per person
- Privacy by default
- Self-hosted over hosted
- Single binary deployment
- Irreplaceable usefulness as the end goal

Praxis is theory becoming action.
