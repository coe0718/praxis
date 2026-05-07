//! Multi-process agent architecture (Spacebot-inspired).
//!
//! Five specialized processes for better fault isolation:
//! - Channel: message routing
//! - Branch: context assembly
//! - Worker: tool execution
//! - Compactor: memory
//! - Corrector: error recovery

use std::sync::Arc;
use tokio::sync::mpsc;

/// Result of async tool execution.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub summary: String,
}

/// Process message types (non-serialized for channel communication).
pub enum ProcessMessage {
    /// Execute tool with correlation ID for response lookup.
    ExecuteTool {
        tool: String,
        args: serde_json::Value,
        correlation_id: String,
    },
    /// Compact context.
    CompactContext {
        context_id: String,
        max_tokens: usize,
    },
    /// Correct error.
    ErrorCorrection {
        error: String,
        context: serde_json::Value,
    },
}

/// Channel process - handles message routing.
pub struct ChannelProcess;

impl ChannelProcess {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn route(&self, channel: &str) -> anyhow::Result<()> {
        log::info!("Channel process: routing to {}", channel);
        Ok(())
    }
}

/// Worker process - handles tool execution.
pub struct WorkerProcess {
    sender: mpsc::Sender<ProcessMessage>,
}

impl WorkerProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self {
        Self { sender }
    }

    pub async fn execute_tool(&self, tool: &str, args: serde_json::Value) -> anyhow::Result<()> {
        let correlation_id = format!("tool-{}", chrono::Utc::now().timestamp_millis());
        self.sender
            .send(ProcessMessage::ExecuteTool {
                tool: tool.to_string(),
                args,
                correlation_id,
            })
            .await?;
        Ok(())
    }

    pub async fn execute_tool_with_result(
        &self,
        tool: &str,
        args: serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let correlation_id = format!("tool-{}", chrono::Utc::now().timestamp_millis());
        self.sender
            .send(ProcessMessage::ExecuteTool {
                tool: tool.to_string(),
                args,
                correlation_id: correlation_id.clone(),
            })
            .await?;
        // Result will be stored by the worker loop - for now return a placeholder
        Ok(ToolResult {
            success: true,
            summary: format!("Tool {} submitted for execution", tool),
        })
    }
}

/// Compactor process - manages memory/consolidation.
pub struct CompactorProcess {
    sender: mpsc::Sender<ProcessMessage>,
}

impl CompactorProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self {
        Self { sender }
    }

    pub async fn compact(&self, context_id: &str, max_tokens: usize) -> anyhow::Result<()> {
        self.sender
            .send(ProcessMessage::CompactContext {
                context_id: context_id.to_string(),
                max_tokens,
            })
            .await?;
        Ok(())
    }
}

/// Corrector process - handles error recovery.
pub struct CorrectorProcess {
    sender: mpsc::Sender<ProcessMessage>,
}

impl CorrectorProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self {
        Self { sender }
    }

    pub async fn correct(&self, error: &str, context: serde_json::Value) -> anyhow::Result<()> {
        self.sender
            .send(ProcessMessage::ErrorCorrection {
                error: error.to_string(),
                context,
            })
            .await?;
        Ok(())
    }
}

/// Process manager - spawns and coordinates all processes.
pub struct ProcessManager {
    worker_tx: mpsc::Sender<ProcessMessage>,
    compactor_tx: mpsc::Sender<ProcessMessage>,
    corrector_tx: mpsc::Sender<ProcessMessage>,
}

impl ProcessManager {
    /// Create a new ProcessManager with the given tool execution callback.
    pub fn with_tool_executor<F>(execute_tool_fn: F) -> Self
    where
        F: Fn(String, serde_json::Value) -> ToolResult + Send + Sync + 'static,
    {
        let (worker_tx, mut worker_rx) = mpsc::channel::<ProcessMessage>(100);
        let (compactor_tx, mut compactor_rx) = mpsc::channel::<ProcessMessage>(100);
        let (corrector_tx, mut corrector_rx) = mpsc::channel::<ProcessMessage>(100);

        // Shared callback wrapped in Arc for the async loop
        let tool_fn = Arc::new(execute_tool_fn);

        // Spawn worker process - actual tool execution
        tokio::spawn(async move {
            while let Some(msg) = worker_rx.recv().await {
                if let ProcessMessage::ExecuteTool { tool, args, .. } = msg {
                    log::info!("Worker process: executing tool {}", tool);
                    let result = tool_fn(tool.clone(), args);
                    log::info!(
                        "Worker process: tool {} completed (success: {})",
                        tool,
                        result.success
                    );
                }
            }
        });

        // Spawn compactor process
        tokio::spawn(async move {
            while let Some(msg) = compactor_rx.recv().await {
                if let ProcessMessage::CompactContext { context_id, .. } = msg {
                    log::info!("Compactor process: compacting context {}", context_id);
                }
            }
        });

        // Spawn corrector process
        tokio::spawn(async move {
            while let Some(msg) = corrector_rx.recv().await {
                if let ProcessMessage::ErrorCorrection { error, .. } = msg {
                    log::info!("Corrector process: handling error: {}", error);
                }
            }
        });

        Self {
            worker_tx,
            compactor_tx,
            corrector_tx,
        }
    }

    /// Create a new ProcessManager with a no-op tool executor.
    pub fn new() -> Self {
        Self::with_tool_executor(|_tool, _args| ToolResult {
            success: true,
            summary: "No-op tool execution".to_string(),
        })
    }

    pub fn worker(&self) -> WorkerProcess {
        WorkerProcess::new(self.worker_tx.clone())
    }

    pub fn channel(&self) -> ChannelProcess {
        ChannelProcess::new()
    }

    pub fn compactor(&self) -> CompactorProcess {
        CompactorProcess::new(self.compactor_tx.clone())
    }

    pub fn corrector(&self) -> CorrectorProcess {
        CorrectorProcess::new(self.corrector_tx.clone())
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ChannelProcess {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_manager_default() {
        let pm = ProcessManager::default();
        let _worker = pm.worker();
        let _channel = pm.channel();
    }

    #[test]
    fn test_channel_process_route() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let channel = ChannelProcess::new();
            channel.route("test").await.unwrap();
        });
    }
}