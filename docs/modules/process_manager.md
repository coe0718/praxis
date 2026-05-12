# Multi-Process Agent Architecture

> Five specialized processes for better fault isolation — Channel (message routing), Worker (tool execution), Compactor (memory), Corrector (error recovery), Branch (context assembly).

## Overview

The process manager module implements a multi-process agent architecture inspired by Spacebot. It provides five specialized process types that communicate via Tokio `mpsc` channels for fault isolation:

- **ChannelProcess** — Message routing (placeholder routing logic).
- **WorkerProcess** — Tool execution; sends `ProcessMessage::ExecuteTool` messages via channel.
- **CompactorProcess** — Memory/context consolidation; sends `ProcessMessage::CompactContext` messages.
- **CorrectorProcess** — Error recovery; sends `ProcessMessage::ErrorCorrection` messages.
- **ProcessManager** — Orchestrator that spawns background Tokio tasks for each process type and provides accessor methods.

The `ProcessManager::with_tool_executor()` constructor spawns three Tokio tasks (worker, compactor, corrector) that listen on their respective channels and log received messages. The tool executor callback is wrapped in an `Arc` for shared access.

**Current status:** Data types, channel-based process communication, and task spawning are implemented. Actual message handling (tool execution, context compaction, error correction) is placeholder — the spawned tasks log received messages but don't perform real work yet.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ProcessMessage` | Enum: `ExecuteTool`, `CompactContext`, `ErrorCorrection` with relevant payloads. |
| `ToolResult` | Result of async tool execution: `{ success, summary }`. |
| `ProcessManager` | Orchestrator with tokio::spawn'd loops and channel senders. |
| `WorkerProcess` | Tool execution client (has sender to worker channel). |
| `ChannelProcess` | Message routing component. |
| `CompactorProcess` | Memory compaction client (has sender to compactor channel). |
| `CorrectorProcess` | Error recovery client (has sender to corrector channel). |

### Channel Architecture

```
ProcessManager
  ├── worker_tx ──→ tokio::spawn loop ──→ tool_fn(args) log
  ├── compactor_tx ──→ tokio::spawn loop ──→ log compaction msg
  └── corrector_tx ──→ tokio::spawn loop ──→ log error msg
```

Each process returns a client handle (WorkerProcess, CompactorProcess, etc.) that sends messages into the corresponding channel.

## Public API

### `ProcessManager`

```rust
impl ProcessManager {
    pub fn new() -> Self
    pub fn with_tool_executor<F>(execute_tool_fn: F) -> Self
        where F: Fn(String, serde_json::Value) -> ToolResult + Send + Sync + 'static
    pub fn worker(&self) -> WorkerProcess
    pub fn channel(&self) -> ChannelProcess
    pub fn compactor(&self) -> CompactorProcess
    pub fn corrector(&self) -> CorrectorProcess
}
```

- **`new`** — Creates channels with capacity 100 but does NOT spawn Tokio tasks. Use for contexts without a Tokio runtime.
- **`with_tool_executor`** — Creates channels AND spawns three async Tokio tasks. Accepts a tool execution callback (`Fn(String, serde_json::Value) -> ToolResult`).
- **`worker`/`channel`/`compactor`/`corrector`** — Returns the respective process client.

### `ProcessMessage`

```rust
pub enum ProcessMessage {
    ExecuteTool {
        tool: String,
        args: serde_json::Value,
        correlation_id: String,
    },
    CompactContext {
        context_id: String,
        max_tokens: usize,
    },
    ErrorCorrection {
        error: String,
        context: serde_json::Value,
    },
}
```

### `ToolResult`

```rust
pub struct ToolResult {
    pub success: bool,
    pub summary: String,
}
```

### Process Clients

```rust
impl WorkerProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self
    pub async fn execute_tool(&self, tool: &str, args: serde_json::Value) -> anyhow::Result<()>
    pub async fn execute_tool_with_result(&self, tool: &str, args: serde_json::Value)
        -> anyhow::Result<ToolResult>
}

impl CompactorProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self
    pub async fn compact(&self, context_id: &str, max_tokens: usize) -> anyhow::Result<()>
}

impl CorrectorProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self
    pub async fn correct(&self, error: &str, context: serde_json::Value) -> anyhow::Result<()>
}

impl ChannelProcess {
    pub fn new() -> Self
    pub async fn route(&self, channel: &str) -> anyhow::Result<()>
}
```

## Configuration

No `praxis.toml` fields. Configured programmatically:

```rust
use praxis::process_manager::{ProcessManager, ToolResult};

// Create a process manager with spawned Tokio tasks
let pm = ProcessManager::with_tool_executor(|tool, args| {
    log::info!("Executing tool: {}", tool);
    ToolResult { success: true, summary: format!("{} done", tool) }
});

let worker = pm.worker();
worker.execute_tool("web_fetch", serde_json::json!({"url": "https://example.com"})).await;

let compactor = pm.compactor();
compactor.compact("session_123", 4000).await;

let corrector = pm.corrector();
corrector.correct("timeout", serde_json::json!({"tool": "web_fetch"})).await;
```

## Dependencies

- **`tokio::sync::mpsc`** — Async multi-producer, single-consumer channels (capacity 100).
- **`std::sync::Arc`** — Shared tool executor callback.

## Status

- ✅ Process message types (ExecuteTool, CompactContext, ErrorCorrection)
- ✅ ProcessManager with tokio::spawn'd background loops
- ✅ Channel-based communication (Worker, Compactor, Corrector)
- ✅ ChannelProcess routing stub
- ✅ with_tool_executor for custom tool integration
- ✅ Default constructor (no runtime required, channels only)
- ⏳ Real processing in spawned loops (currently logs only)
- ⏳ Response correlation (correlation_id → result lookup)
- ⏳ Branch process (context assembly)

## Source

`src/process_manager.rs`