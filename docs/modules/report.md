# Report

> Session status report builder and renderer

## Overview

The `report` module builds and renders a comprehensive status report that summarizes the current state of a Praxis instance. It pulls data from nearly every subsystem — sessions, approvals, evaluations, provider usage, token accounting, learning, drift analysis, canary records, boundary reviews, events, and heartbeat — into a single `StatusReport` struct that can be rendered as a human-readable key-value text block or serialized as JSON.

This is the backend for the `praxis status` CLI command and the `/status` API endpoint.

## Architecture

### Key Types

- **`StatusReport`** — The top-level report struct containing all aggregated fields.
- **`LastSessionReport`** — Session number, outcome, and end time of the most recent session.
- **`LastEvalReport`** — Pass/fail/skip counts and trust-failure count from the latest eval run.
- **`LastProviderUsageReport`** — Provider attempt count, failure count, last provider name, tokens used, and estimated cost.
- **`OperationalMemoryReport`** — Counts of "do not repeat" entries and known bugs.
- **`LastTokenSummaryReport`** — Total tokens and estimated cost.
- **`LastTokenHotspotReport`** — The phase/provider combination with the highest token usage.
- **`HeartbeatReport`** — Current phase and last update timestamp.

### Key Functions

| Function | Purpose |
|---|---|
| `build_status_report(config, paths)` | Aggregate data from all subsystems into a `StatusReport` |
| `render_status_report(report)` | Render the report as a multi-line key-value text block |

## Public API

```rust
pub fn build_status_report(config: &AppConfig, paths: &PraxisPaths) -> Result<StatusReport>
pub fn render_status_report(report: &StatusReport) -> String
```

## Configuration

No direct configuration. The report reflects the current state of the instance as configured by `praxis.toml` and accumulated data.

## Usage

### CLI

```bash
# Display the full status report
praxis status

# JSON output for programmatic consumption
praxis status --json
```

### Programmatic

```rust
let report = report::build_status_report(&config, &paths)?;
let text = report::render_status_report(&report);
println!("{text}");
```

## Data Files

The report reads from:

| Source | Data |
|---|---|
| `session_state.json` | Current phase, last outcome, selected tool |
| SQLite database | Sessions, approvals, anatomy, quality, evals, provider usage, learning |
| `model_canary_file` | Canary record count |
| `boundary_review_file` | Review due status |
| `events_file` | Event count and last event |
| `heartbeat_file` | Heartbeat phase and timestamp |

## Dependencies

- `argus` — Drift analysis
- `boundaries` — Boundary review state
- `canary` — Canary ledger
- `config` — `AppConfig`
- `events` — Event reading
- `heartbeat` — Heartbeat reading
- `learning` — Learning run data
- `state` — `SessionState`, `SessionPhase`
- `storage` — All store traits (`SessionStore`, `ApprovalStore`, `QualityStore`, `ProviderUsageStore`, `OperationalMemoryStore`, `AnatomyStore`)
- `time` — `Clock`, `SystemClock`
- `tools` — `FileToolRegistry`, `ToolRegistry`

## Source

`src/report.rs` + `src/report/render.rs`
