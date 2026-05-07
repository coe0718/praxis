//! (#49) Per-model analytics accumulator.
//!
//! `ModelStats` tracks request count, latency, token usage, and error rate
//! for each model that the backend has routed to.  The accumulator is
//! thread-safe (`Arc<Mutex<..>>`) so it can be shared across the dashboard
//! server and the backend execution path.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Per-model aggregate statistics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ModelStatsEntry {
    /// Model identifier (e.g. `claude-3-5-sonnet-latest`).
    pub model: String,
    /// Total completed requests (both successful and failed).
    pub total_requests: u64,
    /// Total number of failed requests.
    pub error_count: u64,
    /// Cumulative response latency in milliseconds across all requests.
    pub total_latency_ms: u64,
    /// Cumulative input tokens consumed.
    pub total_input_tokens: i64,
    /// Cumulative output tokens produced.
    pub total_output_tokens: i64,
}

impl ModelStatsEntry {
    /// Average latency in milliseconds.  Returns 0.0 when no requests have
    /// been recorded.
    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.total_requests as f64
    }

    /// Error rate as a fraction 0.0 – 1.0.
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.error_count as f64 / self.total_requests as f64
    }
}

/// Thread-safe accumulator shared across the process.
#[derive(Debug, Clone, Default)]
pub struct ModelStats {
    inner: Arc<Mutex<HashMap<String, ModelStatsEntry>>>,
}

impl ModelStats {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a successful request.
    pub fn record_success(
        &self,
        model: &str,
        latency_ms: u64,
        input_tokens: i64,
        output_tokens: i64,
    ) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(model.to_string()).or_default();
        entry.model = model.to_string();
        entry.total_requests += 1;
        entry.total_latency_ms += latency_ms;
        entry.total_input_tokens += input_tokens;
        entry.total_output_tokens += output_tokens;
    }

    /// Record a failed request.
    pub fn record_failure(&self, model: &str, latency_ms: u64) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(model.to_string()).or_default();
        entry.model = model.to_string();
        entry.total_requests += 1;
        entry.error_count += 1;
        entry.total_latency_ms += latency_ms;
    }

    /// Snapshot all entries as a sorted vec (by total_requests descending).
    pub fn snapshot(&self) -> Vec<ModelStatsEntry> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut entries: Vec<_> = map.values().cloned().collect();
        entries.sort_by_key(|e| std::cmp::Reverse(e.total_requests));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_success_and_failure() {
        let stats = ModelStats::new();
        stats.record_success("claude-3-sonnet", 120, 500, 200);
        stats.record_success("claude-3-sonnet", 80, 300, 100);
        stats.record_failure("claude-3-sonnet", 200);

        let snap = stats.snapshot();
        assert_eq!(snap.len(), 1);
        let e = &snap[0];
        assert_eq!(e.total_requests, 3);
        assert_eq!(e.error_count, 1);
        assert_eq!(e.total_latency_ms, 400);
        assert_eq!(e.total_input_tokens, 800);
        assert_eq!(e.total_output_tokens, 300);
        assert!((e.avg_latency_ms() - 133.333).abs() < 0.01);
        assert!((e.error_rate() - 0.333).abs() < 0.01);
    }

    #[test]
    fn multiple_models_sorted_by_requests() {
        let stats = ModelStats::new();
        stats.record_success("model-a", 50, 100, 50);
        stats.record_success("model-b", 60, 100, 50);
        stats.record_success("model-b", 70, 100, 50);

        let snap = stats.snapshot();
        assert_eq!(snap[0].model, "model-b");
        assert_eq!(snap[1].model, "model-a");
    }
}
