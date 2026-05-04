# Observability

> Langfuse integration for LLM tracing, spans, and observability.

## Overview

The observability module provides structured tracing of LLM interactions via [Langfuse](https://langfuse.com). When configured, every LLM call made during the Orient-Decide-Act-Reflect loop can be captured as a trace with input, output, timing, and custom metadata. This gives operators visibility into prompt construction, model selection, response quality, and latency — all in an external Langfuse dashboard.

The module is intentionally lightweight: it defines configuration, trace data structures, and serialization. The actual ingestion to Langfuse's API is handled externally (typically via the Langfuse SDK or HTTP ingestion). Within Praxis, the `LangfuseTrace` struct serves as the canonical representation of a single LLM interaction span.

## Architecture

### Types

| Type | Description |
|------|-------------|
| `LangfuseConfig` | Configuration: public key, secret key, base URL, sample rate. Stored in `praxis.toml` under `[langfuse]`. |
| `LangfuseTrace` | A single trace event: ID, name, input/output JSON, metadata, start/end timestamps in epoch milliseconds. |

`LangfuseConfig` is a serializable struct that integrates with Praxis's `AppConfig`. The `is_enabled()` method returns `true` only when both `public_key` and `secret_key` are set, allowing the rest of the codebase to gate tracing behind a single check.

`LangfuseTrace` uses millisecond-precision timestamps (`i64` epoch millis). A new trace is created with `LangfuseTrace::new("name")` which records the start time. When the LLM call completes, `.complete(output_json)` records the output and end time in a builder-pattern style.

## Public API

```rust
use crate::observability::{LangfuseConfig, LangfuseTrace};

// Configuration
let config = LangfuseConfig::default();
config.is_enabled(); // false — no keys set

// Create and complete a trace
let trace = LangfuseTrace::new("plan_action");
let output = serde_json::json!({"action": "commit_code"});
let completed = trace.complete(output);
// completed.id == "trace_1709123456789"
// completed.start_time is set, completed.end_time is set
```

## Configuration

### praxis.toml

```toml
[langfuse]
public_key = "pk-..."
secret_key = "sk-..."
base_url = "https://cloud.langfuse.com"  # default; change for self-hosted
sample_rate = 1.0  # 0.0–1.0; fraction of traces to capture
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `public_key` | `Option<String>` | `None` | Langfuse project public key |
| `secret_key` | `Option<String>` | `None` | Langfuse project secret key |
| `base_url` | `String` | `https://cloud.langfuse.com` | API base URL (change for self-hosted Langfuse) |
| `sample_rate` | `f64` | `1.0` | Sampling rate (0.0–1.0). Lower values capture a random subset of traces. |

### Environment variables

No dedicated env vars — configuration is loaded from `praxis.toml`. However, the Langfuse SDK itself typically reads `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` if used.

### Feature flags

No feature flag required — always compiled.

## Usage

The observability module is primarily consumed internally by the agent loop. To enable tracing:

1. Add your Langfuse keys to `praxis.toml` under `[langfuse]`.
2. Restart the Praxis daemon.
3. Traces will appear in your Langfuse project dashboard.

There is no CLI command for this module — it operates passively during agent execution.

## Data Files

No local data files. Trace data is sent to the Langfuse API endpoint configured in `base_url`.

## Dependencies

- **`serde` / `serde_json`** — serialization of config and trace structures.
- No dependency on other Praxis modules.

## Source

`src/observability/`
