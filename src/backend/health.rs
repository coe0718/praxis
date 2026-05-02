use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::Mutex,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Tracks provider health for auto-failover decisions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderHealth {
    /// Provider name -> health status.
    pub providers: HashMap<String, ProviderStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    /// Whether the provider is currently healthy.
    pub healthy: bool,
    /// Last time the provider was checked.
    pub last_checked: Option<String>,
    /// Last error message if unhealthy.
    pub last_error: Option<String>,
    /// Number of consecutive failures.
    pub consecutive_failures: u32,
    /// Last successful response time in milliseconds.
    pub last_response_ms: Option<u64>,
}

impl Default for ProviderStatus {
    fn default() -> Self {
        Self {
            healthy: true,
            last_checked: None,
            last_error: None,
            consecutive_failures: 0,
            last_response_ms: None,
        }
    }
}

/// In-memory provider health tracker with periodic persistence.
pub struct ProviderHealthTracker {
    health: Mutex<ProviderHealth>,
    persistence_path: Option<std::path::PathBuf>,
    last_persist: Mutex<Instant>,
}

impl ProviderHealthTracker {
    pub fn new(persistence_path: Option<&Path>) -> Result<Self> {
        let health = if let Some(path) = persistence_path {
            if path.exists() {
                let raw = fs::read_to_string(path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                serde_json::from_str(&raw)
                    .with_context(|| format!("failed to parse {}", path.display()))?
            } else {
                ProviderHealth::default()
            }
        } else {
            ProviderHealth::default()
        };

        Ok(Self {
            health: Mutex::new(health),
            persistence_path: persistence_path.map(|p| p.to_path_buf()),
            last_persist: Mutex::new(Instant::now()),
        })
    }

    /// Record a successful provider response.
    pub fn record_success(&self, provider: &str, response_ms: u64) {
        let mut health = self.health.lock().unwrap();
        let status = health.providers.entry(provider.to_string()).or_default();
        status.healthy = true;
        status.last_error = None;
        status.consecutive_failures = 0;
        status.last_response_ms = Some(response_ms);
        status.last_checked = Some(chrono::Utc::now().to_rfc3339());
        drop(health);
        self.maybe_persist();
    }

    /// Record a provider failure.
    pub fn record_failure(&self, provider: &str, error: &str) {
        let mut health = self.health.lock().unwrap();
        let status = health.providers.entry(provider.to_string()).or_default();
        status.consecutive_failures += 1;
        status.last_error = Some(error.to_string());
        status.last_checked = Some(chrono::Utc::now().to_rfc3339());

        // Mark as unhealthy after 3 consecutive failures
        if status.consecutive_failures >= 3 {
            status.healthy = false;
        }

        drop(health);
        self.maybe_persist();
    }

    /// Check if a provider is healthy.
    pub fn is_healthy(&self, provider: &str) -> bool {
        let health = self.health.lock().unwrap();
        health.providers.get(provider).map(|s| s.healthy).unwrap_or(true) // Assume healthy if no data
    }

    /// Get provider health status.
    pub fn get_status(&self, provider: &str) -> Option<ProviderStatus> {
        let health = self.health.lock().unwrap();
        health.providers.get(provider).cloned()
    }

    /// Get all provider statuses.
    pub fn get_all_statuses(&self) -> HashMap<String, ProviderStatus> {
        let health = self.health.lock().unwrap();
        health.providers.clone()
    }

    /// Reset a provider to healthy status.
    pub fn reset_provider(&self, provider: &str) {
        let mut health = self.health.lock().unwrap();
        if let Some(status) = health.providers.get_mut(provider) {
            status.healthy = true;
            status.consecutive_failures = 0;
            status.last_error = None;
            status.last_checked = Some(chrono::Utc::now().to_rfc3339());
        }
        drop(health);
        self.maybe_persist();
    }

    /// Maybe persist to disk (debounced to avoid excessive I/O).
    fn maybe_persist(&self) {
        let mut last_persist = self.last_persist.lock().unwrap();
        if last_persist.elapsed() < Duration::from_secs(60) {
            return;
        }
        *last_persist = Instant::now();
        drop(last_persist);

        if let Some(path) = &self.persistence_path {
            let health = self.health.lock().unwrap();
            if let Ok(raw) = serde_json::to_string_pretty(&*health) {
                let _ = fs::write(path, raw);
            }
        }
    }
}

/// Global provider health tracker instance.
static PROVIDER_HEALTH: once_cell::sync::Lazy<ProviderHealthTracker> =
    once_cell::sync::Lazy::new(|| {
        // Try to load from default path, or create a new one
        let path = std::env::var("PRAXIS_PROVIDER_HEALTH_FILE").ok().map(std::path::PathBuf::from);
        ProviderHealthTracker::new(path.as_deref())
            .unwrap_or_else(|_| ProviderHealthTracker::new(None).unwrap())
    });

/// Get the global provider health tracker.
pub fn provider_health() -> &'static ProviderHealthTracker {
    &PROVIDER_HEALTH
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_provider_health_tracking() {
        let tracker = ProviderHealthTracker::new(None).unwrap();

        // Initially healthy
        assert!(tracker.is_healthy("openai"));

        // Record success
        tracker.record_success("openai", 150);
        assert!(tracker.is_healthy("openai"));

        // Record failures
        for _ in 0..3 {
            tracker.record_failure("openai", "rate limited");
        }
        assert!(!tracker.is_healthy("openai"));

        // Reset
        tracker.reset_provider("openai");
        assert!(tracker.is_healthy("openai"));
    }

    #[test]
    fn test_provider_health_persistence() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("health.json");

        let tracker = ProviderHealthTracker::new(Some(&path)).unwrap();
        tracker.record_success("openai", 100);
        tracker.record_failure("claude", "timeout");

        // Force persistence
        tracker.maybe_persist();

        // Load from disk
        let tracker2 = ProviderHealthTracker::new(Some(&path)).unwrap();
        assert!(tracker2.is_healthy("openai"));
        assert!(tracker2.is_healthy("claude")); // Only 1 failure, still healthy
    }
}
