# CLI

> Command-line interface — all `praxis` subcommands, clap dispatch, and argument definitions.

## Overview

The `cli` module defines Praxis's entire CLI surface using `clap`. It provides over 40 subcommands covering initialization, agent execution, session management, memory, learning, quality, evolution, messaging, and operational utilities. The module parses arguments via `clap::Parser`, dispatches to handler functions, and returns human-readable string output.

The top-level `Cli` struct accepts an optional `--data-dir` global flag to override the data directory, followed by a `Commands` enum that enumerates every subcommand. The `run()` function is the main entry point, and `execute()` performs the actual dispatch.

## Architecture

### Entry Points

| Function | Description |
|----------|-------------|
| `run()` | Parses CLI args, calls `execute()`, prints output. |
| `execute()` | Matches the `Commands` variant and dispatches to the appropriate handler. |

### Top-Level Structure

```rust
pub struct Cli {
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,
    pub command: Commands,
}
```

### Subcommands

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `init` | Initialize a new Praxis instance. | `--name`, `--timezone`, `--security-level` |
| `ask` | One-shot LLM query with optional tool execution. | `--file`, `--tools/--no-tools`, `-z` (one-shot), `--redact-secrets` |
| `run` | Run the agent loop. | `--once`, `--force`, `--task`, `--profile`, `--one-shot`, `--fast`, `--redact-secrets` |
| `status` | Show current agent status. | — |
| `doctor` | Diagnose configuration issues. | — |
| `sessions` | Search session history. | `[query]`, `--limit` |
| `insights` | Show usage insights and trends. | `--days` |
| `chat` | Interactive chat session. | `--model` |
| `agents` | Agent management. | Subcommands |
| `boundaries` | Boundary management. | Subcommands |
| `export` / `import` | Archive operations. | Path args |
| `argus` | Run Argus analytics. | Subcommands |
| `canary` | Canary testing. | Subcommands |
| `learn` | Learning cycle operations. | Subcommands (`run`, `list`, `accept`, `dismiss`, `note`) |
| `mcp` | MCP protocol operations. | Subcommands |
| `model` | Model configuration. | Subcommands |
| `models` | List available models. | Subcommands |
| `forensics` | Forensic analysis. | Subcommands |
| `heartbeat` | Heartbeat operations. | Subcommands |
| `queue` | Show approval queue. | `--all` |
| `approve` | Approve a pending item. | `id`, `--note` |
| `reject` | Reject a pending item. | `id`, `--note` |
| `telegram` | Telegram integration. | Subcommands |
| `serve` | Start the HTTP/Dashboard server. | Args |
| `tools` | Tool management. | Subcommands |
| `wake` | Request an agent wake. | `reason`, `--task`, `--source`, `--urgent` |
| `bench` | Run benchmarks. | `--log` (show previous results) |
| `compact` | Request context compaction. | `--goal` |
| `oauth` | OAuth management. | Subcommands |
| `git` | Git operations. | Subcommands |
| `watchdog` | Watchdog configuration. | Subcommands |
| `brief` | Brief operations. | Subcommands |
| `daemon` | Daemon management. | Subcommands |
| `delegation` | Agent delegation links. | Subcommands |
| `evolve` | Evolution proposal management. | Subcommands (`list`, `show`, `approve`, `reject`, `apply`) |
| `fallback` | Fallback configuration. | Subcommands |
| `hands` | Hands-off mode management. | Subcommands |
| `hooks` | Hook management. | Subcommands |
| `sandbox` | Sandbox operations. | Subcommands |
| `vault` | Vault/secret management. | Subcommands |
| `webhook` | Webhook operations. | Subcommands |
| `memory` | Memory operations. | Subcommands |
| `ephemeral` | Ephemeral session management. | Subcommands |
| `completions` | Generate shell completions. | `shell` (bash, zsh, fish, elvish, powershell) |
| `checkpoint` | Create a checkpoint. | `--label` |
| `rollback` | Roll back to a checkpoint. | `id` |
| `checkpoints` | List all checkpoints. | — |
| `migrate` | Import from Axonix. | `source`, `--dry-run` |
| `worktree` | Worktree management. | Subcommands (`create`, `list`, `remove`, `merge`) |
| `plan` | Execution plan management. | Subcommands (`create`, `list`, `show`, `dry-run`, `remove`) |
| `profile` | Profile management. | Subcommands (`list`, `create`, `switch`, `remove`, `show`) |
| `discord` | Discord integration. *(feature-gated: `discord`)* | Subcommands |
| `slack` | Slack integration. *(feature-gated: `slack`)* | Subcommands |
| `tui` | Terminal UI. *(feature-gated: `tui`)* | Subcommands |
| `vscode` | VS Code integration. | Subcommands |
| `acp` | Start the ACP server. | — |

## Feature Flags

| Flag | Commands gated |
|------|----------------|
| `discord` | `discord` subcommand |
| `slack` | `slack` subcommand |
| `tui` | `tui` subcommand |

## Public API

```rust
pub fn run() -> Result<()>;
```

The `Cli`, `Commands`, and all `*Args` structs are public for use in integration tests.

## Usage

```bash
# Initialize a new instance
praxis init --name "MyAgent" --timezone "US/Pacific"

# Run a single agent cycle
praxis run --once

# Ask a one-shot question with tools
praxis ask -z "Summarize the latest session outcomes"

# Wake the agent with a task
praxis wake "Deploy the latest changes" --task "run deploy script" --urgent

# Check status and insights
praxis status
praxis insights --days 14

# Search session history
praxis sessions "deploy failure" --limit 10

# Manage evolution proposals
praxis evolve list
praxis evolve approve evo-20260414-080000
praxis evolve apply evo-20260414-080000

# Generate shell completions
praxis completions bash > /etc/bash_completion.d/praxis
```

## Configuration

| Flag | Description |
|------|-------------|
| `--data-dir` | Override the data directory path (global, applies to all commands). |

All other flags are per-command as listed above.

## Dependencies

- `clap` with `derive` — argument parsing
- `clap_complete` — shell completion generation
- All Praxis modules — dispatched to from `execute()`

## Source

`src/cli/mod.rs` (and ~30+ subcommand handler files in `src/cli/`)
