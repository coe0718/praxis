//! Langfuse observability integration.
//!
//! Provides LLM tracing and observability via Langfuse.
//! https://langfuse.com
//!
//! #32 Langfuse Observability

use serde::{Deserialize, Serialize};

/// Get current timestamp in milliseconds since epoch.
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Configuration for Langfuse observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangfuseConfig {
    /// Public key for Langfuse project.
    pub public_key: Option<String>,
    /// Secret key for Langfuse project.
    pub secret_key: Option<String>,
    /// Langfuse API base URL (default: https://cloud.langfuse.com).
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// Sampling rate for traces (0.0-1.0).
    #[serde(default)]
    pub sample_rate: f64,
}

fn default_base_url() -> String {
    "https://cloud.langfuse.com".to_string()
}

impl Default for LangfuseConfig {
    fn default() -> Self {
        Self {
            public_key: None,
            secret_key: None,
            base_url: default_base_url(),
            sample_rate: 1.0,
        }
    }
}

impl LangfuseConfig {
    /// Check if Langfuse is configured and enabled.
    pub fn is_enabled(&self) -> bool {
        self.public_key.is_some() && self.secret_key.is_some()
    }
}

/// Langfuse trace event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LangfuseTrace {
    pub id: String,
    pub name: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub start_time: i64,
    pub end_time: Option<i64>,
}

impl LangfuseTrace {
    /// Create a new trace.
    pub fn new(name: &str) -> Self {
        Self {
            id: format!("trace_{}", now_millis()),
            name: name.to_string(),
            input: None,
            output: None,
            metadata: None,
            start_time: now_millis(),
            end_time: None,
        }
    }

    /// Complete the trace with output.
    pub fn complete(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self.end_time = Some(now_millis());
        self
    }
}
