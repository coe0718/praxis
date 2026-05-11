//! Health monitor — periodic health checks with alerting.
//!
//! Registers health checks for subsystems and runs them periodically.
//! Provides a `/health` endpoint for load balancer probes.
//!
//! ```toml
//! [health]
//! check_interval_secs = 60
//! alert_on_failure = true
//! ```

use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Health status of a subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    /// Subsystem is healthy.
    Healthy,
    /// Subsystem is degraded but functional.
    Degraded { reason: String },
    /// Subsystem is unhealthy.
    Unhealthy { reason: String },
    /// Subsystem hasn't been checked yet.
    Unknown,
}

/// Result of a health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Name of the subsystem.
    pub name: String,
    /// Current status.
    pub status: HealthStatus,
    /// Time the check was performed.
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// Duration of the check in milliseconds.
    pub duration_ms: u64,
    /// Optional details.
    pub details: Option<serde_json::Value>,
}

/// A health check function signature.
pub type HealthCheckFn = Box<dyn Fn() -> HealthStatus + Send + Sync>;

/// Registered health check.
struct RegisteredCheck {
    name: String,
    check_fn: HealthCheckFn,
    last_result: Option<HealthCheckResult>,
    consecutive_failures: u32,
    alert_threshold: u32,
}

/// Health monitor that tracks subsystem health.
pub struct HealthMonitor {
    checks: Mutex<Vec<RegisteredCheck>>,
    start_time: Instant,
}

impl HealthMonitor {
    /// Create a new health monitor.
    pub fn new() -> Self {
        Self {
            checks: Mutex::new(Vec::new()),
            start_time: Instant::now(),
        }
    }

    /// Register a health check.
    pub fn register(&self, name: &str, alert_threshold: u32, check_fn: HealthCheckFn) {
        let mut checks = self.checks.lock().unwrap();
        checks.push(RegisteredCheck {
            name: name.to_string(),
            check_fn,
            last_result: None,
            consecutive_failures: 0,
            alert_threshold,
        });
    }

    /// Run all health checks and return results.
    pub fn check_all(&self) -> Vec<HealthCheckResult> {
        let mut checks = self.checks.lock().unwrap();
        let mut results = Vec::new();

        for registered in checks.iter_mut() {
            let start = Instant::now();
            let status = (registered.check_fn)();
            let duration_ms = start.elapsed().as_millis() as u64;

            match &status {
                HealthStatus::Healthy | HealthStatus::Unknown => {
                    registered.consecutive_failures = 0;
                }
                HealthStatus::Degraded { .. } | HealthStatus::Unhealthy { .. } => {
                    registered.consecutive_failures += 1;
                    if registered.consecutive_failures >= registered.alert_threshold {
                        log::warn!(
                            "health: {} has failed {} consecutive checks (threshold: {})",
                            registered.name,
                            registered.consecutive_failures,
                            registered.alert_threshold,
                        );
                    }
                }
            }

            let result = HealthCheckResult {
                name: registered.name.clone(),
                status,
                checked_at: chrono::Utc::now(),
                duration_ms,
                details: None,
            };
            registered.last_result = Some(result.clone());
            results.push(result);
        }

        results
    }

    /// Get overall system health.
    pub fn overall_health(&self) -> HealthStatus {
        let checks = self.checks.lock().unwrap();
        if checks.is_empty() {
            return HealthStatus::Healthy;
        }

        let mut any_degraded = false;
        for check in checks.iter() {
            if let Some(ref result) = check.last_result {
                match &result.status {
                    HealthStatus::Unhealthy { .. } => return result.status.clone(),
                    HealthStatus::Degraded { .. } => any_degraded = true,
                    _ => {}
                }
            }
        }

        if any_degraded {
            HealthStatus::Degraded {
                reason: "One or more subsystems degraded".to_string(),
            }
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get the last check results (without re-running).
    pub fn last_results(&self) -> Vec<HealthCheckResult> {
        let checks = self.checks.lock().unwrap();
        checks.iter().filter_map(|c| c.last_result.clone()).collect()
    }

    /// Uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Get count of registered checks.
    pub fn check_count(&self) -> usize {
        self.checks.lock().unwrap().len()
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healthy_check() {
        let monitor = HealthMonitor::new();
        monitor.register("test", 3, Box::new(|| HealthStatus::Healthy));

        let results = monitor.check_all();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, HealthStatus::Healthy);
        assert_eq!(results[0].name, "test");
    }

    #[test]
    fn test_overall_health_all_healthy() {
        let monitor = HealthMonitor::new();
        monitor.register("a", 3, Box::new(|| HealthStatus::Healthy));
        monitor.register("b", 3, Box::new(|| HealthStatus::Healthy));
        monitor.check_all();

        assert_eq!(monitor.overall_health(), HealthStatus::Healthy);
    }

    #[test]
    fn test_overall_health_degraded() {
        let monitor = HealthMonitor::new();
        monitor.register("a", 3, Box::new(|| HealthStatus::Healthy));
        monitor.register(
            "b",
            3,
            Box::new(|| HealthStatus::Degraded { reason: "slow".to_string() }),
        );
        monitor.check_all();

        match monitor.overall_health() {
            HealthStatus::Degraded { .. } => {}
            other => panic!("Expected Degraded, got {:?}", other),
        }
    }

    #[test]
    fn test_overall_health_unhealthy() {
        let monitor = HealthMonitor::new();
        monitor.register("a", 3, Box::new(|| HealthStatus::Healthy));
        monitor.register(
            "b",
            3,
            Box::new(|| HealthStatus::Unhealthy { reason: "down".to_string() }),
        );
        monitor.check_all();

        match monitor.overall_health() {
            HealthStatus::Unhealthy { .. } => {}
            other => panic!("Expected Unhealthy, got {:?}", other),
        }
    }

    #[test]
    fn test_uptime() {
        let monitor = HealthMonitor::new();
        assert!(monitor.uptime_secs() < 1);
    }

    #[test]
    fn test_empty_monitor_is_healthy() {
        let monitor = HealthMonitor::new();
        assert_eq!(monitor.overall_health(), HealthStatus::Healthy);
        assert_eq!(monitor.check_count(), 0);
    }

    #[test]
    fn test_last_results_caching() {
        let monitor = HealthMonitor::new();
        monitor.register("test", 3, Box::new(|| HealthStatus::Healthy));
        monitor.check_all();

        let cached = monitor.last_results();
        assert_eq!(cached.len(), 1);
    }
}
