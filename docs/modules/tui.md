# TUI (Terminal Dashboard)

> Real-time terminal dashboard for monitoring a running Praxis agent, built with ratatui and crossterm.

## Overview

The TUI module provides a live, full-screen terminal interface that displays the current state of a Praxis agent at a glance. It renders a four-panel dashboard: a status panel showing the current phase, goal, heartbeat, tool count, and approval queue depth; a last-action panel; a recent-events feed; and a header/footer bar.

The dashboard polls Praxis's data files (session state, heartbeat, events log, approval store) every 750 ms and re-renders the terminal. This makes it useful for watching the agent loop in real time without digging through log files.

The module is **feature-gated** behind the `tui` Cargo feature and is not compiled by default, keeping the binary lean for server deployments that don't need an interactive terminal.

## Architecture

### Layout

```
┌──────────────────────────────────────────────────────────┐
│  Praxis — live dashboard                                  │  Header
├────────────────────────────┬─────────────────────────────┤
│  Status                    │  Recent Events               │
│  phase:     act            │  [complete] tool executed     │
│  outcome:   success        │  [blocked] budget exceeded   │
│  goal:      42: Fix bug    │  [error] LLM timeout         │
│  heartbeat: act @ 10:30    │  ...                         │
│  tools:     4              │                               │
│  queue:     2 pending      │                               │
├────────────────────────────┤                               │
│  Last Action               │                               │
│  Committed fix to main     │                               │
├────────────────────────────┴─────────────────────────────┤
│  q / Ctrl-C to quit   refreshes every 750ms               │  Footer
└──────────────────────────────────────────────────────────┘
```

### Key types

| Type | Description |
|------|-------------|
| `TuiState` | Aggregated snapshot of agent state: phase, goal, outcome, action summary, heartbeat, tool count, pending approvals, and recent events. Constructed by reading files from `PraxisPaths`. |

### Rendering

The module uses `ratatui`'s declarative layout system with `CrosstermBackend`:
- **Header** — static title bar.
- **Status panel** (left, 45% width) — phase color-coded (orient=blue, decide=yellow, act=green, reflect=magenta, sleep=gray), outcome, goal, heartbeat timestamp, tool count, approval queue (yellow if non-zero).
- **Last Action** (left, bottom) — wraps the action summary text.
- **Recent Events** (right, 55% width) — last 12 events from the event log, color-coded by keywords (red for errors, green for success, yellow for blocked/budget).
- **Footer** — keybinding hint.

### Event color coding

| Pattern | Color |
|---------|-------|
| `error`, `fail` | Red |
| `complete`, `success` | Green |
| `blocked`, `budget` | Yellow |
| Everything else | White |

## Public API

```rust
use crate::tui::run_tui;

// Launch the dashboard (blocks until user presses q or Ctrl-C)
run_tui(data_dir_path)?;
```

The function handles raw mode setup, alternate screen, and terminal restoration automatically.

## Configuration

### Feature flag

```toml
# Cargo.toml — enable the TUI at build time
[features]
tui = ["ratatui", "crossterm"]
```

The `tui` feature must be enabled at compile time. Without it, neither the TUI module nor the `praxis tui` CLI command is available.

### CLI

```bash
praxis tui
```

No additional flags or options at this time (future: `--refresh-ms` to control the polling interval).

## Usage

1. Build Praxis with the `tui` feature: `cargo build --features tui`
2. Ensure the Praxis agent is running (the dashboard reads its data files).
3. Run `praxis tui` to open the dashboard.
4. Press `q` or `Ctrl-C` to exit.

The dashboard is read-only — it does not send commands to the agent. It purely observes the state files the daemon writes.

## Data Files

The TUI reads the following files (all under the configured `data_dir`):

| File | Purpose |
|------|---------|
| `session_state.json` | Current phase, goal, outcome, action summary. |
| `heartbeat` | Agent heartbeat with phase and timestamp. |
| `events.jsonl` | Recent event log entries. |
| `praxis.db` (SQLite) | Approval store — counts pending approvals. |
| `tools/` | Tool manifest directory — counts registered tools. |

No files are written by the TUI.

## Dependencies

### Internal Praxis modules

- **`paths`** — `PraxisPaths` for locating data files.
- **`state`** — `SessionState::load()` for current session info.
- **`heartbeat`** — `read_heartbeat()` for agent liveness.
- **`events`** — `read_events_since()` for the event feed.
- **`storage`** — `SqliteSessionStore` for approval queries.
- **`tools`** — `FileToolRegistry` for tool enumeration.

### External crates

- **`ratatui`** — declarative terminal UI framework.
- **`crossterm`** — cross-platform terminal control (raw mode, alternate screen, key events).

## Source

`src/tui/`
