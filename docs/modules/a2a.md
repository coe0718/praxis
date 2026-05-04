# A2A (Agent-to-Agent)

> Google A2A protocol implementation for cross-agent task delegation and discovery.

## Overview

The A2A module implements Google's Agent-to-Agent protocol specification, enabling Praxis to delegate tasks to remote autonomous agents and (in future) accept inbound tasks from other agents. The protocol standardizes how agents discover each other via **Agent Cards**, exchange **Messages** containing structured parts (text, files, data), and track work through a well-defined **Task lifecycle**.

Praxis currently implements the **client side** — outbound task delegation. The `A2aClient` can discover a remote agent's capabilities, send tasks, poll for status, and cancel work. The type system is fully defined for both client and server roles, making it straightforward to add inbound task acceptance in the future.

## Architecture

### Agent discovery

| Type | Description |
|------|-------------|
| `AgentCard` | Public metadata an agent exposes: name, description, URL, version, capabilities, and authentication scheme. |
| `AgentCapabilities` | Feature flags: `streaming`, `push_notifications`, `state_transition_history`. |
| `AuthScheme` | Tagged enum: `None`, `ApiKey { location }`, `OAuth2 { authority }`. |

Agent cards are fetched from the well-known endpoint `/.well-known/agent.json` on the remote agent's URL.

### Task lifecycle

| Type | Description |
|------|-------------|
| `Task` | A task with ID, session ID, status, optional artifact, and optional history of status updates. |
| `TaskStatus` | Enum: `Submitted` → `Working` → `InputRequired` / `Completed` / `Cancelled` / `Failed`. |
| `TaskStatusUpdate` | A status transition with a message and optional timestamp. |

### Messages and artifacts

| Type | Description |
|------|-------------|
| `Message` | A message with a `role` string and a list of `Part` values. |
| `Part` | Tagged enum: `Text { text }`, `File { name, mime_type, bytes, uri }`, `Data { data: Value }`. |
| `Artifact` | A result artifact: list of parts, index, optional append/last_chunk flags, optional metadata. |

### Request/response types

| Type | Direction | Description |
|------|-----------|-------------|
| `SendTaskRequest` | Client → Agent | Contains task ID, session ID, and initial message. |
| `SendTaskResponse` | Agent → Client | Returns task status and optional artifact. |
| `GetTaskRequest` | Client → Agent | Poll for task status with optional `history_length`. |
| `CancelTaskRequest` | Client → Agent | Cancel a running task. |
| `CancelTaskResponse` | Agent → Client | Confirms cancellation with final status. |

## Public API

### A2aClient

```rust
use crate::a2a::{A2aClient, AgentCard, Message, Part, SendTaskRequest};

// Create a client pointing to a remote agent
let client = A2aClient::new("http://remote-agent:3000")?;

// Discover the remote agent's capabilities
let card: AgentCard = client.fetch_agent_card()?;

// Send a task
let req = SendTaskRequest {
    id: "task-001".to_string(),
    session_id: "session-abc".to_string(),
    message: Message {
        role: "user".to_string(),
        parts: vec![Part::Text { text: "Summarize today's metrics".to_string() }],
    },
};
let response = client.send_task(&req)?;

// Poll for status
let task = client.get_task("task-001")?;

// Cancel if needed
let cancel_response = client.cancel_task("task-001")?;
```

The client uses a 120-second HTTP timeout and communicates via JSON over HTTP. The remote agent is expected to implement the A2A spec endpoints at `/tasks/send`, `/tasks/get`, and `/tasks/cancel`.

## Configuration

The A2A module is a library component with no dedicated configuration file. Connection details (remote agent URLs) are provided at runtime via `A2aClient::new()`.

### Feature flags

No feature flag required — always compiled.

## Usage

There is no dedicated CLI command for A2A. The module is used programmatically within Praxis's delegation system and by other modules that need to coordinate with remote agents.

To test connectivity to a remote A2A-compatible agent, you can use the client in a custom tool or integration.

## Data Files

No local data files. The A2A module is stateless — task state is managed by the remote agent.

## Dependencies

- **`reqwest`** (blocking) — HTTP client for A2A endpoint calls.
- **`serde` / `serde_json`** — JSON serialization for all A2A types.
- No dependency on other Praxis modules.

## Source

`src/a2a/`
