# Paths

> A single struct holding every file path Praxis uses — the canonical map of the data directory.

## Overview

The paths module centralizes all file-path resolution into one struct: `PraxisPaths`. Instead of scattering `data_dir.join("some_file.json")` calls across the entire codebase, every module receives a reference to `PraxisPaths` and accesses paths through typed fields. This eliminates typos, makes path changes a single-line edit, and provides a clear inventory of every file Praxis reads or writes.

`PraxisPaths` is constructed from a data directory root and derives all sub-paths from it. Two convenience constructors — `for_data_dir()` and `from_config()` — cover the most common initialization patterns. The module also provides `default_data_dir()` for resolving the platform-appropriate default location when no `--data-dir` flag is given.

## Architecture

### `PraxisPaths` Field Groups

The struct has 50+ fields organized into logical groups:

**Core configuration:**
`config_file`, `providers_file`, `profiles_file`, `budgets_file`, `security_file`

**Identity files:**
`soul_file`, `identity_file`, `operator_model_file`, `goals_file`, `roadmap_file`, `journal_file`, `metrics_file`, `patterns_file`, `learnings_file`, `agents_file`, `capabilities_file`, `proposals_file`, `day_count_file`

**Session and state:**
`database_file`, `state_file`, `events_file`, `heartbeat_file`, `score_file`

**Context and memory:**
`context_adaptation_file`, `context_cache_file`, `memory_conflicts_file`, `user_memory_file`, `todo_file`, `context_groups_file`, `system_anomalies_file`

**Messaging and activation:**
`bus_file`, `activation_file`, `telegram_state_file`, `sender_pairing_file`, `discord_state_file`, `slack_state_file`

**Tools and hands:**
`tools_dir`, `hands_dir`, `active_hand_file`, `tool_cooldowns_file`

**Skills and learning:**
`skills_dir`, `skills_drafts_dir`, `learning_dir`, `learning_sources_dir`

**Delegation and scheduling:**
`delegation_links_file`, `delegation_queue_file`, `scheduled_jobs_file`, `cron_outputs_dir`

**Security and vault:**
`vault_file`, `master_key_file`, `sandbox_file`, `canary_freeze_file`, `route_weights_file`

**Daemon and hooks:**
`daemon_pid_file`, `hooks_file`, `webhooks_file`, `watchdog_update_file`

**Evolution and quality:**
`evolution_file`, `self_evolution_file`, `model_canary_file`, `boundary_review_file`

**Backups and benchmarks:**
`backups_dir`, `benchmarks_dir`, `benchmark_log_file`

**Directories:**
`data_dir`, `goals_dir`, `goals_criteria_dir`, `evals_dir`

### Platform Detection

```rust
pub fn detect_platform() -> Platform   // Linux, MacOs, Other
pub fn default_data_dir() -> Result<PathBuf>
pub fn default_data_dir_for(platform, home, xdg) -> PathBuf
```

| Platform | Default Path |
|---|---|
| Linux | `$XDG_DATA_HOME/praxis` or `~/.local/share/praxis` |
| macOS | `~/Library/Application Support/Praxis` |
| Other | `~/.praxis` |

## Public API

### Constructors

```rust
PraxisPaths::for_data_dir(data_dir: PathBuf) -> Self
PraxisPaths::from_config(config: &AppConfig) -> Self
PraxisPaths::new(data_dir: PathBuf, database_path: PathBuf, state_file: PathBuf) -> Self
```

`for_data_dir()` uses defaults for database (`praxis.db`) and state file (`session_state.json`). `from_config()` reads path overrides from `AppConfig`.

### Helpers

```rust
paths.identity_files() -> Vec<PathBuf>
```

Returns the list of identity-related files (soul, identity, goals, roadmap, journal, etc.) used by anatomy refresh and validation.

## Usage

Every module that needs file access receives `&PraxisPaths`:

```rust
let paths = PraxisPaths::for_data_dir(data_dir);
let config = AppConfig::load(&paths.config_file)?;
let state = SessionState::load(&paths.state_file)?;
```

## Data Files

This module defines the names and locations of **all** data files used by Praxis. See the `PraxisPaths` struct fields for the complete list.

## Dependencies

- **config** — `AppConfig` for `from_config()` constructor

## Source

`src/paths.rs`
