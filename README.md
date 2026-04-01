# Praxis

Praxis is a standalone, self-evolving personal AI agent framework written in Rust.

It is designed to be self-hosted, private by default, and shaped around one operator instead of a generic user profile. The long-term goal is not to build another chatbot. The goal is to build an always-on personal agent that learns, develops capabilities, and becomes genuinely irreplaceable over time.

The reference implementation is Axonix. Praxis is the framework extraction of those ideas into a reusable codebase.

## Current status

This repository is in the foundation milestone.

Implemented in the first round:

- Rust crate scaffold with a library-first structure
- `praxis` CLI with `init`, `run --once`, `status`, and `doctor`
- TOML config loading and validation
- Cross-platform data directory handling for Linux and macOS
- Resumable session state with the `orient -> decide -> act -> reflect -> sleep` loop
- SQLite-backed session storage
- Markdown-based identity and goal files
- Deterministic offline tests
- Dockerfile and `docker-compose.yml` for containerized runs

Not implemented yet:

- Live Claude integration
- Messaging platforms
- Memory consolidation and FTS search
- SSE/dashboard consumers
- Watchdog updates
- Reviewer sub-agents and success-criteria execution

## Foundation commands

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

## Project shape

The current codebase is organized around small modules with a preference for keeping Rust source files under 250 lines.

- `src/cli.rs`: CLI surface and command dispatch
- `src/config.rs`: typed config schema and validation
- `src/paths.rs`: data directory and path resolution
- `src/state.rs`: persisted session checkpoint state
- `src/identity/`: markdown identity and goal parsing/policy
- `src/storage/`: SQLite session persistence
- `src/loop/`: runtime loop orchestration
- `tests/cli.rs`: end-to-end CLI coverage

## Design direction

The broader Praxis vision includes:

- hot and cold memory tiers
- context budgeting and handoff notes
- tool registry and approval queues
- messaging interfaces
- dashboard and SSE streaming
- reviewer sub-agents
- a watchdog-managed update path

The detailed design and long-form architecture notes live in [PRAXIS_DESIGN.md](./PRAXIS_DESIGN.md).

## Development

Run formatting and tests locally:

```bash
cargo fmt
cargo test
```

## Philosophy

- One instance per person
- Privacy by default
- Self-hosted over hosted
- Single binary deployment
- Irreplaceable usefulness as the end goal

Praxis is theory becoming action.
