# Channels

> Signal/Matrix channels — Enterprise messaging support. Adds secure enterprise messaging channels wired into the Praxis event system.

## Overview

The `channels` module provides enterprise messaging integrations for Signal and Matrix. Both clients are constructed from environment variables and follow the same pattern: build from environment (return `None` if unconfigured), then send messages via the configured channel.

The `SignalClient` uses a synchronous `send()` method (currently a stub that logs messages but does not actually transmit). The `MatrixClient` uses an async `send()` method that makes an HTTP POST to a Matrix homeserver's REST API (`/_matrix/client/r0/rooms/{roomId}/send/m.room.message`) with bearer token authentication and URL-encoded room IDs (C6 security fix).

A top-level `notify_event()` free function wires both channels into the event system — it constructs transient clients from environment variables and forwards significant events (delegated, task complete, approval required, etc.) to configured channels.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `SignalClient` | Signal messenger client. Reads `PRAXIS_SIGNAL_PHONE` from environment. |
| `MatrixClient` | Matrix client. Reads `PRAXIS_MATRIX_HOMESERVER`, `PRAXIS_MATRIX_ROOM`, `PRAXIS_MATRIX_TOKEN`. |

### Environment Variables

| Variable | Client | Required |
|----------|--------|----------|
| `PRAXIS_SIGNAL_PHONE` | Signal | Yes for Signal |
| `PRAXIS_MATRIX_HOMESERVER` | Matrix | Yes for Matrix |
| `PRAXIS_MATRIX_ROOM` | Matrix | Yes for Matrix |
| `PRAXIS_MATRIX_TOKEN` | Matrix | Yes for Matrix |

## Public API

```rust
// Signal client
pub struct SignalClient;
impl SignalClient {
    pub fn from_env() -> Option<Self>;
    pub fn send(&self, recipient: &str, message: &str) -> Result<()>;
}

// Matrix client
pub struct MatrixClient;
impl MatrixClient {
    pub fn from_env() -> Option<Self>;
    pub async fn send(&self, message: &str) -> Result<()>;
}

// Event notification
pub fn notify_event(kind: &str, detail: &str);
```

## Configuration

All configuration is via environment variables. See table above.

### Example

```rust
// Send a Signal notification
if let Some(signal) = SignalClient::from_env() {
    signal.send("operator", "[Praxis] Deployment complete")?;
}

// Send a Matrix notification
if let Some(matrix) = MatrixClient::from_env() {
    matrix.send("[Praxis] Task 'deploy' completed successfully").await?;
}

// Notify via both channels
notify_event("deploy.complete", "Production deployment finished");
```

## Security

- **Matrix room ID**: URL-encoded to prevent injection attacks (C6 fix).
- **Bearer token**: Sent via HTTP `Authorization` header, not in the URL.
- **Signal**: Currently a stub — no actual message is transmitted. Designed for integration with `signal-cli` or a Signal REST API.

## Dependencies

- `reqwest` — Matrix HTTP client
- `urlencoding` — safe room ID encoding
- `chrono` — transaction ID timestamps
- `anyhow` — error handling
- `log` — debug logging
- `serde_json` — Matrix message payload

## Source

`src/channels.rs`