//! Rate limiter — configurable rate limiting for tool execution and API calls.
//!
//! Token bucket algorithm with per-tool, per-provider, and global limits.
//! Configure via `praxis.toml`:
//!
//! ```toml
//! [rate_limit]
//! global_rps = 10           # max requests per second globally
//! burst = 20                # burst capacity
//!
//! [rate_limit.tools]
//! web_search = { rps = 2, burst = 5 }
//! code_exec = { rps = 1, burst = 3 }
//! ```

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Global requests per second.
    #[serde(default = "default_global_rps")]
    pub global_rps: f64,
    /// Global burst capacity.
    #[serde(default = "default_burst")]
    pub burst: u32,
    /// Per-tool limits: tool_name -> { rps, burst }.
    pub tools: HashMap<String, ToolRateLimit>,
}

fn default_global_rps() -> f64 {
    10.0
}
fn default_burst() -> u32 {
    20
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_rps: default_global_rps(),
            burst: default_burst(),
            tools: HashMap::new(),
        }
    }
}

/// Per-tool rate limit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRateLimit {
    pub rps: f64,
    #[serde(default = "default_tool_burst")]
    pub burst: u32,
}

fn default_tool_burst() -> u32 {
    5
}

/// Token bucket for rate limiting.
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Try to consume a token. Returns true if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Time until next token is available.
    fn wait_time(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::ZERO
        } else {
            let deficit = 1.0 - self.tokens;
            Duration::from_secs_f64(deficit / self.refill_rate)
        }
    }

    /// Current token count.
    fn available(&self) -> f64 {
        self.tokens
    }
}

/// Rate limiter with global and per-tool buckets.
pub struct RateLimiter {
    global: Mutex<TokenBucket>,
    tools: Mutex<HashMap<String, TokenBucket>>,
}

impl RateLimiter {
    /// Create a new rate limiter from config.
    pub fn new(config: &RateLimitConfig) -> Self {
        let global = TokenBucket::new(config.burst as f64, config.global_rps);
        let mut tools = HashMap::new();
        for (name, limit) in &config.tools {
            tools.insert(name.clone(), TokenBucket::new(limit.burst as f64, limit.rps));
        }
        Self {
            global: Mutex::new(global),
            tools: Mutex::new(tools),
        }
    }

    /// Check if a request is allowed for a given tool.
    /// Returns Ok(()) if allowed, Err with wait time if rate limited.
    pub fn check(&self, tool: &str) -> Result<(), Duration> {
        // Check tool-specific limit first
        {
            let mut tools = self.tools.lock().unwrap();
            if let Some(bucket) = tools.get_mut(tool) {
                if !bucket.try_consume() {
                    let wait = bucket.wait_time();
                    return Err(wait);
                }
            }
        }

        // Check global limit
        {
            let mut global = self.global.lock().unwrap();
            if !global.try_consume() {
                let wait = global.wait_time();
                return Err(wait);
            }
        }

        Ok(())
    }

    /// Get current available capacity for a tool.
    pub fn available(&self, tool: &str) -> (f64, f64) {
        let global_avail = {
            let g = self.global.lock().unwrap();
            g.available()
        };
        let tool_avail = {
            let tools = self.tools.lock().unwrap();
            tools.get(tool).map(|b| b.available()).unwrap_or(f64::MAX)
        };
        (global_avail, tool_avail)
    }

    /// Register a new tool with custom rate limits.
    pub fn register_tool(&self, name: &str, rps: f64, burst: u32) {
        let mut tools = self.tools.lock().unwrap();
        tools.insert(name.to_string(), TokenBucket::new(burst as f64, rps));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let config = RateLimitConfig {
            global_rps: 100.0,
            burst: 100,
            tools: HashMap::new(),
        };
        let limiter = RateLimiter::new(&config);

        for _ in 0..10 {
            assert!(limiter.check("test_tool").is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_blocks_on_exhaust() {
        let config = RateLimitConfig {
            global_rps: 0.1, // very slow refill
            burst: 2,
            tools: HashMap::from([("expensive".to_string(), ToolRateLimit { rps: 0.1, burst: 1 })]),
        };
        let limiter = RateLimiter::new(&config);

        assert!(limiter.check("expensive").is_ok());
        // Second should fail (burst=1)
        assert!(limiter.check("expensive").is_err());
    }

    #[test]
    fn test_register_tool() {
        let config = RateLimitConfig::default();
        let limiter = RateLimiter::new(&config);
        limiter.register_tool("custom", 5.0, 10);

        for _ in 0..10 {
            assert!(limiter.check("custom").is_ok());
        }
    }
}
