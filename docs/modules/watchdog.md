# Watchdog

> Two-process supervisor that manages the Praxis agent lifecycle, handles auto-updates, and performs safe binary rollback.

## Overview

The watchdog module is Praxis's self-management layer. It serves two roles:

1. **A standalone supervisor binary** (`praxis-watchdog`) that owns the cron schedule, spawns the main `praxis` binary as a child process, monitors its exit status, and handles safe update/rollback cycles.
2. **A set of CLI commands** (`praxis watchdog ...`) for installing the agent as a system service, checking health, downloading updates, and rolling back failed updates.

The two-process architecture is deliberate: the watchdog binary is intentionally simple and **never updates itself**. Only the main `praxis` binary is swapped during updates. If a new binary causes crashes, the watchdog rolls back to the previous version automatically.

## Architecture

### Supervisor binary (`praxis-watchdog`)

The standalone watchdog binary runs a simple async loop:

1. On each tick (configurable interval, default 60 seconds):
   - Write a heartbeat timestamp to `watchdog_heartbeat`.
   - Spawn `praxis --data-dir <dir> run --once` as a child process.
   - Wait for the child to exit.
   - If the child exits with an error and a recent update record exists, attempt rollback.
2. On `Ctrl-C`, shut down cleanly.

The watchdog locates the `praxis` binary by looking in the same directory as itself, falling back to PATH lookup.

### Update lifecycle

```
praxis watchdog update --check     # Check for newer release (no download)
praxis watchdog update             # Download new binary to temp file
praxis watchdog update --apply     # Download + backup + replace + record
praxis watchdog rollback           # Restore previous binary from backup
```

When `--apply` is used:
1. **Data backup** — the entire `data_dir` is archived to a gzip tarball in `backups/` (unless `backup_before_update = false` in config).
2. **Binary backup** — the current executable is copied to `backups/praxis.prev`.
3. **Binary replacement** — the downloaded binary replaces the current executable.
4. **Record** — `watchdog_update.json` records the version, timestamp, and backup path.

### Service installation

| Platform | Mechanism | Details |
|----------|-----------|---------|
| Linux | systemd | Creates a `.service` unit + `.timer` unit. User-level by default; system-level with `--user` flag. |
| macOS | launchd | Creates a `.plist` in `~/Library/LaunchAgents/` with `StartInterval`. |

### Heartbeat monitoring

The `praxis watchdog check` command reads the agent's heartbeat file and compares its timestamp against the current time. If the heartbeat is older than `--max-age-secs` (default 900 seconds / 15 minutes), the agent is considered stalled. With `--restart`, it spawns a one-shot `praxis run --once` to recover.

On stall detection, failing canary routes are automatically frozen to prevent the agent from getting stuck on a broken model provider.

### Key types

| Type | Description |
|------|-------------|
| `WatchdogUpdateRecord` | JSON record of an applied update: version, timestamp, backup path. |
| `UpdateRecord` | Internal to the supervisor: previous and current binary paths. |
| `WatchdogArgs` / `WatchdogCommand` | CLI argument parsing. |

## Public API

This module is primarily consumed through CLI commands, not programmatically.

## Configuration

### CLI flags

#### Supervisor binary (`praxis-watchdog`)

```
praxis-watchdog [--data-dir <path>] [--check-interval <seconds>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--data-dir` | `/var/lib/praxis` | Praxis data directory. |
| `--check-interval` | `60` | Seconds between session spawns. |

#### CLI commands (`praxis watchdog ...`)

```bash
# Install as a system service
praxis watchdog install [--interval-secs 300] [--user <username>]

# Uninstall the service
praxis watchdog uninstall

# Show service status
praxis watchdog status

# Heartbeat check
praxis watchdog check [--max-age-secs 900] [--restart]

# Update
praxis watchdog update [--repo coe0718/praxis] [--check] [--apply]

# Rollback to previous binary
praxis watchdog rollback
```

### praxis.toml

```toml
[runtime]
backup_before_update = true  # Set to false to skip data_dir backup during updates
```

### Platform support

- **Linux** — systemd (user or system level)
- **macOS** — launchd
- Other platforms are not supported for service installation.

## Usage

### First-time setup

```bash
# Install the agent as a system service (runs every 5 minutes)
praxis watchdog install --interval-secs 300

# Check service status
praxis watchdog status
```

### Updating

```bash
# Check if an update is available
praxis watchdog update --check

# Download and apply the update
praxis watchdog update --apply

# If something went wrong, roll back
praxis watchdog rollback
```

### Monitoring

```bash
# Check if the agent heartbeat is recent
praxis watchdog check

# Check and restart if stalled
praxis watchdog check --restart --max-age-secs 600
```

## Data Files

| File | Location | Description |
|------|----------|-------------|
| `watchdog_update.json` | `{data_dir}/watchdog_update.json` | Record of the last applied update (version, timestamp, backup path). Deleted on rollback. |
| `watchdog_heartbeat` | `{data_dir}/watchdog_heartbeat` | RFC 3339 timestamp written by the supervisor on each tick. |
| `watchdog.log` | `{data_dir}/watchdog.log` | stdout/stderr from the supervisor's child process runs. |
| `backups/praxis.prev` | `{data_dir}/backups/praxis.prev` | Backup of the previous `praxis` binary. |
| `backups/praxis-data-*.tar.gz` | `{data_dir}/backups/` | Data directory archive taken before each update (if `backup_before_update` is enabled). |

## Dependencies

### Internal Praxis modules

- **`paths`** — `PraxisPaths` for locating data files.
- **`config`** — `AppConfig` for the `backup_before_update` flag.
- **`heartbeat`** — `check_heartbeat()` for stall detection.
- **`canary`** — `ModelCanaryLedger`, `CanaryFreezeState` for freezing failing routes on stall.
- **`time`** — `SystemClock` for heartbeat age comparison.

### External crates

- **`tokio`** — async runtime for the supervisor binary.
- **`reqwest`** (blocking) — GitHub API calls for update checks.
- **`flate2` + `tar`** — data directory backup compression.
- **`serde` / `serde_json`** — serialization of update records and GitHub API responses.

## Source

`src/watchdog/` (supervisor binary) and `src/cli/watchdog.rs` (CLI commands)
