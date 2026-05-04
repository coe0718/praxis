# Dashboard

> HTTP API server with SSE events, Prometheus metrics, and an OpenAI-compatible chat endpoint.

## Overview

The dashboard module is Praxis's web control plane — an Axum-based HTTP server that exposes the agent's internal state, control mechanisms, and real-time event stream to operators and external tools. It serves as the primary integration point for web UIs, monitoring systems, and third-party automation.

The server is organized into seven route modules, each responsible for a coherent slice of functionality:

- **routes_core** — Read-only views of sessions, goals, identity files, tools, models, config, heartbeat, scores, reports, canary state, token usage, and health checks.
- **routes_control** — Action endpoints: wake the agent, run a task, ask a question, trigger a canary run, approve evolution proposals, inspect delegation links, and an OpenAI-compatible `/v1/chat/completions` endpoint.
- **routes_events** — SSE event stream, Discord/Slack webhook receivers, dynamic webhook handler, Prometheus metrics, MCP endpoint, and static pages.
- **routes_memory** — Hot/cold memory listing, search, consolidation, reinforce/forget operations, and approval queue management.
- **routes_learning** — List learning opportunities, add notes, run the learning engine, accept/dismiss opportunities.
- **routes_admin** — Agents file management, vault secrets, boundary rules and confirmation, forensics snapshots, and Argus analysis.
- **model_stats** — Thread-safe per-model analytics accumulator for request counts, latency, token usage, and error rates.

Authentication is via a `Bearer` token set in the `PRAXIS_DASHBOARD_TOKEN` environment variable. SSE and webhook routes are public (Discord/Slack send webhooks without tokens; EventSource cannot send headers).

## Architecture

### Key Types

| Type | Module | Description |
|------|--------|-------------|
| `DashboardState` | server | Shared Axum state: data directory path, optional auth token, Discord public key, Slack signing secret. |
| `ModelStats` | model_stats | Thread-safe accumulator (`Arc<Mutex<HashMap<String, ModelStatsEntry>>>`). |
| `ModelStatsEntry` | model_stats | Per-model aggregate: total requests, errors, latency, input/output tokens. |
| `ApprovalRow` | types | Serialisable approval record for API responses. |
| `MemoryRow` | types | Serialisable memory record for API responses. |
| `GoalRow` | types | Serialisable goal record for API responses. |
| `ChatCompletionsRequest` | types | OpenAI-compatible request body with multi-modal content support. |
| `ChatCompletionsResponse` | types | OpenAI-compatible response body. |

### Route Organization

All routes are mounted under the root path. The `serve_dashboard` function builds an Axum `Router` with authentication middleware and CSP security headers.

**Public routes** (no auth):
- `GET /events` — SSE stream
- `POST /webhook/discord` — Discord interaction webhook (feature-gated)
- `POST /webhook/slack` — Slack Events API webhook (feature-gated)
- `POST /webhook/:name` — Dynamic webhook subscriptions

**Authenticated routes** (require `Bearer` token):
- Core: `/api/summary`, `/api/sessions`, `/api/goals`, `/api/identity/:file`, `/api/config`, `/api/tools`, `/api/models`, `/api/heartbeat`, `/api/score`, `/api/report`, `/api/canary`, `/api/tokens`, `/api/tokens/sessions`, `/api/health`
- Control: `/api/wake`, `/api/run`, `/api/ask`, `/api/canary/run`, `/api/evolution`, `/api/evolution/:id/approve`, `/api/delegation`, `/v1/chat/completions`
- Memory: `/api/memories/hot`, `/api/memories/cold`, `/api/memories/search`, `/api/memories/consolidate`, `/api/memories/:id/reinforce`, `/api/memories/:id/forget`, `/api/approvals`, `/api/approvals/:id/approve`, `/api/approvals/:id/reject`
- Learning: `/api/learning`, `/api/learning/note`, `/api/learning/run`, `/api/learning/:id/accept`, `/api/learning/:id/dismiss`
- Admin: `/api/agents`, `/api/vault`, `/api/vault/:name`, `/api/boundaries`, `/api/boundaries/confirm`, `/api/forensics`, `/api/argus`

## Public API

### Entry Point

```rust
pub async fn serve_dashboard(data_dir: PathBuf, host: String, port: u16) -> Result<()>
```

Starts the Axum HTTP server. Binds to `host:port`, applies auth middleware, and serves all routes.

### ModelStats

```rust
pub fn new() -> Self
pub fn record_success(&self, model: &str, latency_ms: u64, input_tokens: i64, output_tokens: i64)
pub fn record_failure(&self, model: &str, latency_ms: u64)
pub fn snapshot(&self) -> Vec<ModelStatsEntry>
```

Thread-safe accumulator that can be shared across the backend execution path and dashboard server.

## Configuration

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `PRAXIS_DASHBOARD_TOKEN` | Recommended | Bearer token for API authentication. If unset, all endpoints are unauthenticated and a warning is logged. |
| `PRAXIS_DISCORD_PUBLIC_KEY` | Discord webhooks | ED25519 public key for verifying Discord webhook signatures. |
| `PRAXIS_SLACK_SIGNING_SECRET` | Slack webhooks | HMAC-SHA256 signing secret for verifying Slack Events API payloads. |

### CLI

```bash
# Start the dashboard server
praxis serve --host 0.0.0.0 --port 8080
```

Host and port can also be configured in `praxis.toml` under `[dashboard]`.

## Usage

### API Examples

```bash
# Get agent summary
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/summary

# List recent sessions
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/sessions

# Wake the agent with a task
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"task": "Review open PRs", "reason": "operator request"}' \
  http://localhost:8080/api/wake

# Ask a question (OpenAI-compatible)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"messages": [{"role": "user", "content": "What are the current goals?"}]}' \
  http://localhost:8080/v1/chat/completions

# Search memories
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/memories/search?q=authentication"

# SSE event stream (token as query param for EventSource)
# Connect browser EventSource to: http://localhost:8080/events?token=$TOKEN

# Prometheus metrics
curl http://localhost:8080/metrics
```

### Prometheus Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `praxis_hot_memory_count` | gauge | Number of hot memories. |
| `praxis_cold_memory_count` | gauge | Number of cold memories. |
| `praxis_approvals_pending` | gauge | Pending tool-approval requests. |
| `praxis_heartbeat_age_seconds` | gauge | Seconds since last heartbeat (-1 = never). |
| `praxis_sessions_total` | gauge | Total sessions (by last session ID). |
| `praxis_tokens_total` | gauge | Total tokens consumed. |
| `praxis_cost_micros_total` | gauge | Total estimated cost in micro-dollars. |
| `praxis_provider_tokens_total{provider}` | gauge | Tokens consumed per provider. |

## Data Files

The dashboard reads from standard Praxis data files via `PraxisPaths`. It does not write any files directly (except through delegated CLI functions).

| File | Purpose |
|------|---------|
| `data_dir/praxis.db` | SQLite database queried for sessions, memories, approvals, tokens. |
| `data_dir/events.jsonl` | Event log consumed by SSE and `/events/recent`. |
| `data_dir/webhooks.json` | Dynamic webhook subscription store. |
| `data_dir/delegation_links.json` | Delegation link store. |
| Standard identity files | `SOUL.md`, `IDENTITY.md`, `GOALS.md`, `AGENTS.md`, etc. |

## Dependencies

- **`cli`** — Delegates to CLI handlers for `handle_run`, `handle_ask`, `handle_status`, `handle_doctor`, etc.
- **`paths`** — `PraxisPaths` for resolving data file locations.
- **`storage`** — `SqliteSessionStore` for database queries.
- **`memory`** — `MemoryStore` trait for memory operations.
- **`events`** — `read_events_since` for SSE and recent events.
- **`messaging`** — Discord and Slack types for webhook handling.
- **`wakeup`** — `WakeIntent` / `request_wake` for control endpoints.
- **`evolution`** — `EvolutionStore` for proposal management.
- **`delegation`** — `DelegationStore` for delegation inspection.
- **`learning`** — Opportunity listing, note appending, run trigger.
- **`vault`** — `Vault` for secret management.
- **`boundaries`** — Boundary listing, addition, confirmation.
- **`forensics`** — Snapshot loading.
- **`argus`** — Report analysis and rendering.
- **`canary`** — Canary ledger and freeze state.
- **`heartbeat`** — Heartbeat reading.
- **`mcp`** — MCP dispatch for the `/mcp` endpoint.
- **`config`** — `AppConfig` loading.
- **`backend`** — `ConfiguredBackend` for the chat completions endpoint.
- **`usage`** — `UsageBudgetPolicy` for budget checking.

## Source

`src/dashboard/`
