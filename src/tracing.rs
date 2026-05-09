//! OpenTelemetry and structured logging setup.
//!
//! Provides:
//! - JSON-formatted logs (via tracing_subscriber)
//! - Prometheus metrics endpoint

use anyhow::Result;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing_subscriber::{EnvFilter, fmt};

/// Global metrics registry for Prometheus exports.
pub mod metrics {
    use super::*;

    pub static REQUEST_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
    pub static TOOL_EXECUTION_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
    pub static MCP_CALL_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));
    pub static STREAMING_TOKEN_COUNT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

    pub fn inc_request() {
        REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_tool() {
        TOOL_EXECUTION_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_mcp() {
        MCP_CALL_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_tokens(n: u64) {
        STREAMING_TOKEN_COUNT.fetch_add(n, Ordering::Relaxed);
    }

    pub fn gather() -> String {
        format!(
            "# HELP praxis_requests_total Total requests processed\n\
             # TYPE praxis_requests_total counter\n\
             praxis_requests_total {}\n\
             # HELP praxis_tool_executions_total Total tool executions\n\
             # TYPE praxis_tool_executions_total counter\n\
             praxis_tool_executions_total {}\n\
             # HELP praxis_mcp_calls_total Total MCP tool calls\n\
             # TYPE praxis_mcp_calls_total counter\n\
             praxis_mcp_calls_total {}\n\
             # HELP praxis_streaming_tokens_total Total streaming tokens\n\
             # TYPE praxis_streaming_tokens_total counter\n\
             praxis_streaming_tokens_total {}\n",
            REQUEST_COUNT.load(Ordering::Relaxed),
            TOOL_EXECUTION_COUNT.load(Ordering::Relaxed),
            MCP_CALL_COUNT.load(Ordering::Relaxed),
            STREAMING_TOKEN_COUNT.load(Ordering::Relaxed),
        )
    }
}

/// Initialize tracing with JSON output.
///
/// Call once at program startup. Environment variables:
/// - `PRAXIS_LOG=debug|info|warn|error` — Set log level
pub fn init_tracing() -> Result<()> {
    let log_level = std::env::var("PRAXIS_LOG").unwrap_or_else(|_| "info".to_string());

    let env_filter = EnvFilter::try_new(&log_level).unwrap_or_else(|_| EnvFilter::new("info"));

    // Use try_init to avoid panic when called multiple times (e.g., in parallel tests).
    let _ = fmt().with_env_filter(env_filter).json().try_init();

    Ok(())
}
