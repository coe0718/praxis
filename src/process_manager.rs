//! Multi-process agent architecture (Spacebot-inspired).
//!
//! Five specialized processes for better fault isolation:
//! - Channel: message routing
//! - Branch: context assembly  
//! - Worker: tool execution
//! - Compactor: memory
//! - Corrector: error recovery

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Process message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessMessage {
    /// Route message to channel.
    Channel {
        channel: String,
        payload: serde_json::Value,
        reply_to: Option<String>,
    },
    /// Execute tool.
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
pub struct ChannelProcess {
    sender: mpsc::Sender<ProcessMessage>,
}

impl ChannelProcess {
    pub fn new(sender: mpsc::Sender<ProcessMessage>) -> Self {
        Self { sender }
    }

    pub async fn route(&self, channel: &str, payload: serde_json::Value) -> anyhow::Result<()> {
        self.sender
            .send(ProcessMessage::Channel {
                channel: channel.to_string(),
                payload,
                reply_to: None,
            })
            .await?;
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

    pub async fn execute_tool(
        &self,
        tool: &str,
        args: serde_json::Value,
        correlation_id: &str,
    ) -> anyhow::Result<()> {
        self.sender
            .send(ProcessMessage::ExecuteTool {
                tool: tool.to_string(),
                args,
                correlation_id: correlation_id.to_string(),
            })
            .await?;
        Ok(())
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
    channel_tx: mpsc::Sender<ProcessMessage>,
    worker_tx: mpsc::Sender<ProcessMessage>,
    compactor_tx: mpsc::Sender<ProcessMessage>,
    corrector_tx: mpsc::Sender<ProcessMessage>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let (channel_tx, mut channel_rx) = mpsc::channel::<ProcessMessage>(100);
        let (worker_tx, mut worker_rx) = mpsc::channel::<ProcessMessage>(100);
        let (compactor_tx, mut compactor_rx) = mpsc::channel::<ProcessMessage>(100);
        let (corrector_tx, mut corrector_rx) = mpsc::channel::<ProcessMessage>(100);

        // Spawn channel process
        tokio::spawn(async move {
            while let Some(msg) = channel_rx.recv().await {
                if let ProcessMessage::Channel { channel, payload, .. } = msg {
                    // Route to appropriate channel
                    log::info!("Channel process: routing to {}", channel);
                }
            }
        });

        // Spawn worker process
        tokio::spawn(async move {
            while let Some(msg) = worker_rx.recv().await {
                if let ProcessMessage::ExecuteTool { tool, .. } = msg {
                    log::info!("Worker process: executing tool {}", tool);
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
            channel_tx,
            worker_tx,
            compactor_tx,
            corrector_tx,
        }
    }

    pub fn channel(&self) -> ChannelProcess {
        ChannelProcess::new(self.channel_tx.clone())
    }

    pub fn worker(&self) -> WorkerProcess {
        WorkerProcess::new(self.worker_tx.clone())
    }

    pub fn compactor(&self) -> CompactorProcess {
        CompactorProcess::new(self.compactor_tx.clone())
    }

    pub fn corrector(&self) -> CorrectorProcess {
        CorrectorProcess::new(self.corrector_tx.clone())
    }
}