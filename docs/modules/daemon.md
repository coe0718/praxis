# Daemon

> Always-on two-process supervisor that runs Praxis as a persistent daemon with cron scheduling, reactive triggers, and graceful shutdown.

## Overview

The daemon module wraps the single-session `run_once()` loop in an outer supervisor that keeps Praxis running continuously. Rather than invoking `praxis run --once` on a system cron, the daemon stays alive and manages its own scheduling. It reacts to inbound bus events (Telegram messages, webhooks), proactive wake intents, and a learned operator schedule — triggering sessions only when there is work to do.

The daemon maintains itself between sessions: it refreshes stale anatomy entries during idle periods, hot-reloads `praxis.toml` without restarting, and records operator activity timestamps to build an adaptive quiet-hours schedule. On `SIGTERM` or `Ctrl-C`, it finishes the current Reflect phase before exiting so no session data is lost.

A PID file (`daemon.pid`) prevents double-start. Run under systemd, supervisord, or Docker `restart=always` for production reliability.

## Architecture

### Key Types

| Type | Description |
|---|---|
| `DaemonConfig` | Runtime tuning: poll interval, minimum session gap, maintenance interval, force-start flag |
| `SessionTrigger` | What caused a session: `WakeIntent`, `BusEvent`, or `Scheduled` |
| `PidFile` | RAII guard — writes PID on construction, removes file on drop |
| `BusWatcher` | Tracks `bus.jsonl` byte length to detect new inbound events |
| `OperatorSchedule` | Learned hourly activity histogram for adaptive quiet-hour detection |

### Event Loop

```
run_daemon()
  └─ async_daemon_loop()
       │
       ├─ acquire PidFile (guard against double-start)
       ├─ spawn SIGTERM/SIGINT handler → set shutdown flag
       ├─ start ConfigWatcher for hot-reload
       │
       └─ loop:
            ├─ check shutdown flag → clean exit
            ├─ hot-reload praxis.toml if dirty
            │
            ├─ check reactive triggers (priority order):
            │   1. wake_intent.json
            │   2. bus.jsonl growth
            │   3. scheduled jobs (cron)
            │
            ├─ if no reactive trigger → check scheduled session:
            │   - enough time since last session?
            │   - not in adaptive quiet hours?
            │
            ├─ if triggered → run_session_blocking()
            │   - re-read config from disk
            │   - apply profile settings
            │   - build PraxisRuntime and call run_once()
            │
            ├─ if idle → run_maintenance_blocking()
            │   - refresh stale anatomy entries
            │
            └─ sleep(poll_interval)
```

### Signal Handling

On Unix, both `SIGTERM` and `SIGINT` set a shared `AtomicBool` that the main loop checks. The daemon never aborts mid-session — it always completes the current Reflect phase before exiting. On Windows, only `Ctrl-C` is handled.

### Adaptive Scheduling

The `OperatorSchedule` builds a histogram of hourly interaction counts. After 20+ samples, hours with fewer than 10% of peak activity are treated as quiet and skipped for scheduled sessions. Reactive triggers (wake intents, bus events) always fire regardless.

## Public API

### `run_daemon()`

```rust
pub fn run_daemon(data_dir: PathBuf, config: DaemonConfig) -> Result<()>
```

Main entry point. Blocks until shutdown. Builds a tokio runtime for signal handling and async sleep, then enters the daemon loop.

### Status helpers

```rust
pub fn read_daemon_pid(paths: &PraxisPaths) -> Option<u32>
pub fn daemon_is_running(paths: &PraxisPaths) -> bool
pub fn daemon_status_line(paths: &PraxisPaths) -> String
```

Used by `praxis status` and `praxis daemon status` to report daemon health.

## Configuration

### `DaemonConfig` fields

| Field | Type | Default | Description |
|---|---|---|---|
| `poll_interval_secs` | `u64` | `30` | How often to poll for bus events and wake intents |
| `min_session_gap_mins` | `u64` | `240` (4h) | Minimum gap between scheduled sessions |
| `maintenance_interval_mins` | `u64` | `60` | Gap between idle-time maintenance runs |
| `force_start` | `bool` | `false` | Skip PID-file guard (use with care) |

### CLI flags

```bash
praxis daemon           # start daemon with default config
praxis daemon --force   # skip PID guard
```

### systemd unit example

```ini
[Unit]
Description=Praxis agent daemon
After=network.target

[Service]
ExecStart=/usr/local/bin/praxis daemon
Restart=on-failure
RestartSec=10
Environment=ANTHROPIC_API_KEY=***

[Install]
WantedBy=multi-user.target
```

## Data Files

| File | Purpose |
|---|---|
| `daemon.pid` | PID of the running daemon (RAII-guarded, removed on clean exit) |
| `operator_schedule.json` | Hourly interaction histogram for adaptive quiet-hour detection |
| `scheduled_jobs.json` | Cron-like scheduled jobs checked each poll cycle |
| `cron_outputs/` | Output files from scheduled job runs |
| `bus.jsonl` | Inbound event stream monitored by `BusWatcher` |
| `wake_intent.json` | Ephemeral wake trigger (consumed on read) |

## Dependencies

- **loop** — `PraxisRuntime`, `RunOptions`, `RunSummary`
- **config** — `AppConfig` (re-read every session), `ConfigWatcher` (hot-reload)
- **paths** — `PraxisPaths` for all file locations
- **heartbeat** — `write_heartbeat()` to signal liveness
- **wakeup** — `consume_intent()` for reactive wake triggers, `OperatorSchedule` for adaptive scheduling
- **identity** — `LocalIdentityPolicy`, `MarkdownGoalParser`
- **storage** — `SqliteSessionStore`
- **tools** — `FileToolRegistry`, `sync_capabilities()`
- **lite** — `LiteMode`
- **time** — `SystemClock`
- **profiles** — `ProfileSettings` (applied each session)
- **events** — `FileEventSink`
- **backend** — `ConfiguredBackend`
- **anatomy** — `refresh_stale_anatomy()` for maintenance
- **plugins** — `PluginRegistry`

## Source

`src/daemon.rs`
