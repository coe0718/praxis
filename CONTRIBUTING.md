# Contributing to Praxis

## Prerequisites

- **Rust** 1.80+ (install via [rustup](https://rustup.rs/))
- **SQLite** (dev headers: `libsqlite3-dev` on Debian/Ubuntu, `brew install sqlite3` on macOS)
- **OpenSSL** (dev headers: `libssl-dev` on Debian/Ubuntu)
- **Git** 2.30+
- **Docker** (optional, for containerized testing)

## Code Shape

- Keep Rust code files under 250 lines whenever practical.
- Split modules early instead of letting core files grow past the limit.
- Prefer small focused modules over large multi-responsibility files.

## Foundation Defaults

- Keep the first milestone fully offline and deterministic in tests.
- Preserve Docker support when adding new runtime behavior.

## Building

```bash
# Debug build (fast iteration)
cargo build

# Release build (optimized binary)
cargo build --release

# Quick check (no codegen, no linking)
cargo check
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --test integration

# Run a specific test by name
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests matching a pattern
cargo test delegate

# Build docs without running tests
cargo doc --no-deps
```

## Linting & Formatting

```bash
cargo fmt                     # Auto-format all files
cargo clippy                  # Lint with clippy (must be clean before committing)
cargo clippy -- -D warnings   # Treat warnings as errors (CI mode)
```

## Project Structure

```
src/
  loop/           — Runtime orchestration (orient → decide → act → reflect)
  identity/       — SOUL.md / IDENTITY.md loading and validation
  memory/         — Hot/cold/link memory store traits + vector/LanceDB backends
  storage/        — SQLite trait implementations
  tools/          — Manifest loading, approval queue, execution, cooldowns
  context/        — Context assembly, compaction, handoff notes
  messaging/      — Telegram, Discord, Slack, and event bus
  backend/        — LLM backends (OpenAI, Claude, Ollama, provider routing)
  dashboard/      — Web dashboard (actix-web + static frontend)
  cli/            — All subcommands, including daemon, tui, mcp, skills
  config/         — AppConfig model, validation, file watcher
  delegation/     — Agent-to-agent delegation + A2A fallback
  mcp/            — Model Context Protocol (client + server)
  daemon/         — Daemon lifecycle, signal handling, platform polling
  a2a/            — Agent-to-Agent protocol client + types
  archive/        — Versioned snapshots and bundles
  argus/          — Drift detection and pattern analysis
  kanban/         — Built-in kanban task board
  oauth/          — OAuth device flow (GitHub, Google, Gmail, Copilot)
  plugins/        — Plugin system with marketplace and registry
  skills/         — Runtime skill synthesis
  tui/            — Terminal UI (ratatui-based)
  vault/          — Encrypted credentials storage
  ...             — 35+ single-file modules (pipeline, circuit_breaker, etc.)
docs/
  agent_runtime.md  — Agent loop deep dive
  modules/          — Per-module API reference (92 files)
tests/              — Integration tests (tmp data dir pattern)
```

## Adding a New Module

1. Create `src/your_module.rs` (or `src/your_module/mod.rs` for a directory module)
2. Register it in `src/lib.rs` with `pub mod your_module;`
3. Add a corresponding `docs/modules/your_module.md`
4. Add integration tests in `tests/` or unit tests via `#[cfg(test)]`
5. Run `cargo fmt && cargo clippy && cargo test` before committing

## Commit Convention

- Prefix commits with the domain: `delegation:`, `mcp:`, `context:`, `tools:`, etc.
- Use present tense imperative: "Add streaming response support", not "Added"
- Keep commits atomic (one logical change per commit)

## PR Workflow

1. Run all local checks: `cargo fmt && cargo clippy -- -D warnings && cargo test`
2. Update any relevant module docs if the public API changed
3. Open the PR with a summary of what changed and why
4. Reference the relevant issue or internal tracking ID if applicable