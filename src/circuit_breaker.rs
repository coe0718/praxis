//! Circuit breaker — prevent cascading failures by stopping requests to failing services.
//!
//! Implements the circuit breaker pattern: closed (normal), open (failing), half-open (testing).

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation, requests pass through.
    Closed,
    /// Circuit is open, requests fail fast.
    Open,
    /// Testing if service recovered.
    HalfOpen,
}

/// Configuration for a circuit breaker.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit.
    pub failure_threshold: u32,
    /// Number of successes needed to close from half-open.
    pub success_threshold: u32,
    /// How long to stay open before trying half-open.
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(30),
        }
    }
}

// C7+C8 fix: Consolidated mutable state behind a single Mutex to prevent deadlocks (C8)
// and enable shared state across calls (C7).
struct InnerState {
    state: CircuitState,
    failures: u64,
    successes: u64,
    last_failure: Option<Instant>,
}

/// A circuit breaker instance.
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    inner: Mutex<InnerState>,
}

impl CircuitBreaker {
    pub fn new(name: &str) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    pub fn with_config(name: &str, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            inner: Mutex::new(InnerState {
                state: CircuitState::Closed,
                failures: 0,
                successes: 0,
                last_failure: None,
            }),
        }
    }

    /// Current state of the circuit.
    pub fn state(&self) -> CircuitState {
        let inner = self.inner.lock().unwrap();

        // Check if we should transition from Open to HalfOpen
        if inner.state == CircuitState::Open
            && let Some(t) = inner.last_failure
            && t.elapsed() >= self.config.timeout
        {
            let mut inner = self.inner.lock().unwrap();
            inner.state = CircuitState::HalfOpen;
            return CircuitState::HalfOpen;
        }
        inner.state
    }

    /// Whether a request is allowed.
    pub fn is_available(&self) -> bool {
        matches!(self.state(), CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// Record a successful call.
    pub fn record_success(&self) {
        let mut inner = self.inner.lock().unwrap();

        match inner.state {
            CircuitState::Closed => {
                inner.successes += 1;
            }
            CircuitState::HalfOpen => {
                inner.successes += 1;
                if inner.successes >= self.config.success_threshold as u64 {
                    inner.state = CircuitState::Closed;
                    inner.failures = 0;
                    inner.successes = 0;
                    inner.last_failure = None;
                }
            }
            CircuitState::Open => {} // Shouldn't happen
        }
    }

    /// Record a failed call.
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.failures += 1;
        inner.last_failure = Some(Instant::now());

        match inner.state {
            CircuitState::Closed => {
                if inner.failures >= self.config.failure_threshold as u64 {
                    inner.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                inner.state = CircuitState::Open;
                inner.successes = 0;
            }
            CircuitState::Open => {} // Already open
        }
    }

    /// Reset the circuit breaker to closed state.
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        *inner = InnerState {
            state: CircuitState::Closed,
            failures: 0,
            successes: 0,
            last_failure: None,
        };
    }

    /// Get the name of this circuit.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get failure count.
    pub fn failures(&self) -> u64 {
        self.inner.lock().unwrap().failures
    }
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("CircuitBreaker")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("state", &inner.state)
            .field("failures", &inner.failures)
            .finish()
    }
}

/// Registry of circuit breakers for different services.
#[derive(Debug, Default)]
pub struct CircuitBreakerRegistry {
    // C7 fix: Store Arc<CircuitBreaker> so callers share the same state
    breakers: Mutex<Vec<Arc<CircuitBreaker>>>,
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a circuit breaker. Returns an Arc to shared state.
    pub fn get(&self, name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.lock().unwrap();
        if let Some(cb) = breakers.iter().find(|b| b.name == name) {
            return Arc::clone(cb);
        }

        let cb = Arc::new(CircuitBreaker::new(name));
        breakers.push(Arc::clone(&cb));
        cb
    }

    /// Get all circuit breaker states.
    pub fn states(&self) -> Vec<(String, CircuitState, u64)> {
        let breakers = self.breakers.lock().unwrap();
        breakers
            .iter()
            .map(|b| {
                let inner = b.inner.lock().unwrap();
                (b.name.clone(), inner.state, inner.failures)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closed_state_allows_requests() {
        let cb = CircuitBreaker::new("test");
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.is_available());
    }

    #[test]
    fn test_failure_opens_circuit() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::with_config("test", config);

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.is_available());
    }

    #[test]
    fn test_registry_returns_shared_state() {
        let reg = CircuitBreakerRegistry::new();
        let cb1 = reg.get("api");
        let cb2 = reg.get("api");

        cb1.record_failure();
        assert_eq!(cb2.failures(), 1);
    }
}
