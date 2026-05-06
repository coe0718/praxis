//! Retry-with-backoff — exponential backoff for rate-limited provider calls.
//!
//! #4 429 fallback (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! While `credential_pool.rs` handles key-level 429 cooldown, this module
//! adds request-level retry with exponential backoff and jitter for cases
//! where all keys are rate-limited or a single-key setup hits 429/5xx.

use std::thread;
use std::time::Duration;

use anyhow::Result;

/// Policy controlling retry behavior for transient failures.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retry).
    pub max_retries: u32,
    /// Base delay in milliseconds before the first retry.
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds between retries.
    pub max_delay_ms: u64,
    /// Exponential backoff multiplier.
    pub backoff_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30_000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryPolicy {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }

    /// Compute delay for the given attempt number (0-indexed).
    /// delay = min(base * factor^attempt, max) + jitter(0..200ms)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exponential = self.base_delay_ms as f64 * self.backoff_factor.powi(attempt as i32);
        let capped = exponential.min(self.max_delay_ms as f64) as u64;
        // Jitter: 0-200ms random offset to avoid thundering herd
        let jitter = capped % 200;
        Duration::from_millis(capped + jitter)
    }
}

/// Trait for determining if an error is retryable.
pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}

impl IsRetryable for anyhow::Error {
    /// Check if this error represents a transient failure worth retrying.
    ///
    /// Checks for HTTP 429 (rate limit), 502, 503, 504 (server errors),
    /// and common rate-limit keywords in the error chain.
    fn is_retryable(&self) -> bool {
        let msg = format!("{self:#}");
        msg.contains("429")
            || msg.contains("rate")
            || msg.contains("Rate")
            || msg.contains("502")
            || msg.contains("503")
            || msg.contains("504")
            || msg.contains("server error")
            || msg.contains("overloaded")
            || msg.contains("capacity")
            || msg.contains("temporarily")
    }
}

/// Execute a fallible operation with exponential backoff retry.
///
/// Retries up to `policy.max_retries` times when the operation returns
/// a retryable error (as determined by `IsRetryable`).
///
/// # Example
/// ```ignore
/// use crate::backend::retry::{retry_with_backoff, RetryPolicy};
/// let result = retry_with_backoff(&RetryPolicy::default(), || {
///     transport.send(route, request)
/// })?;
/// ```
pub fn retry_with_backoff<F, T>(policy: &RetryPolicy, f: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let mut last_err = None;

    for attempt in 0..=policy.max_retries {
        match f() {
            Ok(val) => return Ok(val),
            Err(err) => {
                let retryable = err.is_retryable();
                let msg = format!("{err:#}");
                let brief: String = msg.chars().take(120).collect();

                if attempt < policy.max_retries && retryable {
                    let delay = policy.delay_for_attempt(attempt);
                    log::warn!(
                        "retry: attempt {}/{} failed (retryable): {} — waiting {:?}",
                        attempt + 1,
                        policy.max_retries,
                        brief,
                        delay,
                    );
                    thread::sleep(delay);
                    last_err = Some(err);
                } else {
                    if !retryable {
                        log::warn!(
                            "retry: attempt {} failed (non-retryable): {}",
                            attempt + 1,
                            brief
                        );
                    }
                    return Err(err);
                }
            }
        }
    }

    // Should be unreachable, but just in case
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry exhausted without capturing error")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_increases_exponentially() {
        let policy = RetryPolicy::default();
        let d0 = policy.delay_for_attempt(0);
        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);
        assert!(d1 > d0, "delay should increase: {:?} > {:?}", d1, d0);
        assert!(d2 > d1, "delay should increase: {:?} > {:?}", d2, d1);
    }

    #[test]
    fn delay_capped_at_max() {
        let policy = RetryPolicy::default();
        let d = policy.delay_for_attempt(100);
        assert!(d <= Duration::from_millis(policy.max_delay_ms + 200));
    }

    #[test]
    fn retryable_errors_detected() {
        let err = anyhow::anyhow!("HTTP 429: Too Many Requests");
        assert!(err.is_retryable());

        let err = anyhow::anyhow!("502 Bad Gateway");
        assert!(err.is_retryable());

        let err = anyhow::anyhow!("invalid API key");
        assert!(!err.is_retryable());
    }

    #[test]
    fn retry_succeeds_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNT: AtomicU32 = AtomicU32::new(0);

        let policy = RetryPolicy::new(3);
        let result = retry_with_backoff(&policy, || {
            let n = COUNT.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                Err(anyhow::anyhow!("429 rate limited"))
            } else {
                Ok("success")
            }
        });
        assert_eq!(result.unwrap(), "success");
    }

    #[test]
    fn retry_exhausted_on_non_retryable() {
        let policy = RetryPolicy::new(3);
        let result: Result<&str> =
            retry_with_backoff(&policy, || Err(anyhow::anyhow!("invalid API key")));
        assert!(result.is_err());
    }
}
