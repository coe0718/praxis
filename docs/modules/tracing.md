# Tracing

> OpenTelemetry and structured logging setup with Prometheus-compatible metrics.

## Overview

The tracing module initialises structured JSON logging via `tracing_subscriber` and exposes atomic counters for Prometheus-style metrics. It provides a one-shot `init_tracing()` function called at program startup and a `metrics` sub-module with global counters for requests, tool executions, MCP calls, and streaming tokens.

Metrics are gathered into a plain-text Prometheus exposition format string via `metrics::gather()` for serving on an HTTP endpoint.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `metrics::REQUEST_COUNT` | Global `AtomicU64` — total requests processed. |
| `metrics::TOOL_EXECUTION_COUNT` | Global `AtomicU64` — total tool executions. |
| `metrics::MCP_CALL_COUNT` | Global `AtomicU64` — total MCP tool calls. |
| `metrics::STREAMING_TOKEN_COUNT` | Global `AtomicU64` — total streaming tokens. |

## Public API

### `init_tracing`

```rust
pub fn init_tracing() -> Result<()>
```

Initialises `tracing_subscriber` with JSON formatting and an `EnvFilter` controlled by the `PRAXIS_LOG` environment variable (default: `"info"`).

Uses `try_init()` so it can be safely called multiple times (e.g., in parallel tests) without panicking.

### `metrics` Module

```rust
pub mod metrics {
    pub static REQUEST_COUNT: Lazy<AtomicU64>;
    pub static TOOL_EXECUTION_COUNT: Lazy<AtomicU64>;
    pub static MCP_CALL_COUNT: Lazy<AtomicU64>;
    pub static STREAMING_TOKEN_COUNT: Lazy<AtomicU64>;

    pub fn inc_request()
    pub fn inc_tool()
    pub fn inc_mcp()
    pub fn inc_tokens(n: u64)
    pub fn gather() -> String
}
```

- **`inc_request`** — Atomically increments `REQUEST_COUNT`.
- **`inc_tool`** — Atomically increments `TOOL_EXECUTION_COUNT`.
- **`inc_mcp`** — Atomically increments `MCP_CALL_COUNT`.
- **`inc_tokens`** — Atomically adds `n` to `STREAMING_TOKEN_COUNT`.
- **`gather`** — Returns a Prometheus-format string with all four counters and their HELP/TYPE comments.

### Prometheus Output Format

```
# HELP praxis_requests_total Total requests processed
# TYPE praxis_requests_total counter
praxis_requests_total 42
# HELP praxis_tool_executions_total Total tool executions
# TYPE praxis_tool_executions_total counter
praxis_tool_executions_total 17
# HELP praxis_mcp_calls_total Total MCP tool calls
# TYPE praxis_mcp_calls_total counter
praxis_mcp_calls_total 5
# HELP praxis_streaming_tokens_total Total streaming tokens
# TYPE praxis_streaming_tokens_total counter
praxis_streaming_tokens_total 28000
```

## Configuration

Environment variable:

| Variable | Default | Description |
|----------|---------|-------------|
| `PRAXIS_LOG` | `"info"` | Log level for `EnvFilter`. Values: `debug`, `info`, `warn`, `error`. |

## Usage

```rust
use praxis::tracing::{init_tracing, metrics};

// Call once at startup
init_tracing()?;

// Record metrics
metrics::inc_request();
metrics::inc_tool();
metrics::inc_tokens(500);

// Serve metrics endpoint
let prometheus_output = metrics::gather();
```

## Data Files

None. Metrics are in-memory atomics. Logs are written to stdout/stderr via `tracing_subscriber`.

## Dependencies

- **`tracing_subscriber`** — JSON log formatting and env filter.
- **`once_cell`** — `Lazy` static initialisation for metrics counters.
- **`anyhow`** — Error handling.

## Source

`src/tracing.rs`