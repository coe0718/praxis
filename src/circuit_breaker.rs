//! Circuit breaker — prevent cascading failures by stopping requests to failing services.
//!
//! Implements the circuit breaker pattern: closed (normal), open (failing), half-open (testing).

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// A circuit breaker instance.
pub struct CircuitBreaker {
    name: String,
    config: CircuitBreakerConfig,
    state: Mutex<CircuitState>,
    failures: AtomicU64,
    successes: AtomicU64,
    last_failure: Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    pub fn new(name: &str) -> Self {
        Self::with_config(name, CircuitBreakerConfig::default())
    }

    pub fn with_config(name: &str, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            state: Mutex::new(CircuitState::Closed),
            failures: AtomicU64::new(0),
            successes: AtomicU64::new(0),
            last_failure: Mutex::new(None),
        }
    }

    /// Current state of the circuit.
    pub fn state(&self) -> CircuitState {
        let state = *self.state.lock().unwrap();

        // Check if we should transition from Open to HalfOpen
        if state == CircuitState::Open {
            let last = *self.last_failure.lock().unwrap();
            if let Some(t) = last {
                if t.elapsed() >= self.config.timeout {
                    *self.state.lock().unwrap() = CircuitState::HalfOpen;
                    return CircuitState::HalfOpen;
                }
            }
        }
        state
    }

    /// Whether a request is allowed.
    pub fn is_available(&self) -> bool {
        matches!(self.state(), CircuitState::Closed | CircuitState::HalfOpen)
    }

    /// Record a successful call.
    pub fn record_success(&self) {
        let mut state = self.state.lock().unwrap();

        match *state {
            CircuitState::Closed => {
                let _ = self.successes.fetch_add(1, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.successes.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.config.success_threshold as u64 {
                    *state = CircuitState::Closed;
                    self.failures.store(0, Ordering::Relaxed);
                    self.successes.store(0, Ordering::Relaxed);
                    *self.last_failure.lock().unwrap() = None;
                }
            }
            CircuitState::Open => {} // Shouldn't happen
        }
    }

    /// Record a failed call.
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.lock().unwrap() = Some(Instant::now());

        let mut state = self.state.lock().unwrap();
        match *state {
            CircuitState::Closed => {
                if failures >= self.config.failure_threshold as u64 {
                    *state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any failure in half-open goes back to open
                *state = CircuitState::Open;
                self.successes.store(0, Ordering::Relaxed);
            }
            CircuitState::Open => {} // Already open
        }
    }

    /// Reset the circuit breaker to closed state.
    pub fn reset(&self) {
        *self.state.lock().unwrap() = CircuitState::Closed;
        self.failures.store(0, Ordering::Relaxed);
        self.successes.store(0, Ordering::Relaxed);
        *self.last_failure.lock().unwrap() = None;
    }

    /// Get the name of this circuit.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get failure count.
    pub fn failures(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }
}

impl std::fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("name", &self.name)
            .field("config", &self.config)
            .field("state", &self.state())
            .field("failures", &self.failures.load(Ordering::Relaxed))
            .finish()
    }
}

/// Registry of circuit breakers for different services.
#[derive(Debug, Default)]
pub struct CircuitBreakerRegistry {
    breakers: Mutex<Vec<CircuitBreaker>>,
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a circuit breaker.
    pub fn get(&self, name: &str) -> CircuitBreaker {
        let mut breakers = self.breakers.lock().unwrap();
        if let Some(cb) = breakers.iter().find(|b| b.name == name) {
            return CircuitBreaker::with_config(
                &cb.name,
                CircuitBreakerConfig {
                    failure_threshold: cb.config.failure_threshold,
                    success_threshold: cb.config.success_threshold,
                    timeout: cb.config.timeout,
                },
            );
        }

        let cb = CircuitBreaker::new(name);
        breakers.push(CircuitBreaker::new(name)); // Keep a copy
        cb
    }

    /// Get all circuit breaker states.
    pub fn states(&self) -> Vec<(String, CircuitState, u64)> {
        let breakers = self.breakers.lock().unwrap();
        breakers
            .iter()
            .map(|b| {
                let state = CircuitBreaker::new(&b.name).state();
                (b.name.clone(), state, b.failures())
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
        assert!(cb.is_available());

        cb.record_failure();
        assert!(!cb.is_available());
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(50),
        };
        let cb = CircuitBreaker::with_config("test", config);

        cb.record_failure();
        cb.record_failure();
        assert!(!cb.is_available());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_success_closes_circuit() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
        };
        let cb = CircuitBreaker::with_config("test", config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_secs(1),
        };
        let cb = CircuitBreaker::with_config("test", config);

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_registry() {
        let reg = CircuitBreakerRegistry::new();
        let cb = reg.get("api");
        assert_eq!(cb.name(), "api");
    }
}
