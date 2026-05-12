# Rate Limiter

> Configurable rate limiting for tool execution and API calls — token bucket algorithm with per-tool, per-provider, and global limits.

## Overview

The rate limiter module implements the token bucket algorithm for controlling request rates. Each `RateLimiter` maintains a global token bucket plus per-tool buckets. Tokens refill at a configured rate (tokens per second), and requests consume a token when allowed. If no token is available, the request is rejected with the expected wait time.

The architecture supports three tiers of limits: global (across all tools), per-tool (overrides for specific tools), and a fallback for unregistered tools that only hits the global bucket. Configuration is sourced from `praxis.toml` with sensible defaults.

**Current status:** Fully implemented. Used in tool execution to prevent API rate limit violations.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `RateLimitConfig` | Global config: `global_rps` (default 10), `burst` (default 20), plus per-tool limits. |
| `ToolRateLimit` | Per-tool config: `rps` and `burst` (default 5). |
| `TokenBucket` | Internal: tracks tokens, max tokens, refill rate, and last refill time. |
| `RateLimiter` | Public rate limiter with global + per-tool `Mutex<TokenBucket>` buckets. |

### Token Bucket Algorithm

```
At each check:
  1. Refill: tokens += elapsed_seconds * refill_rate (capped at max_tokens)
  2. If tokens >= 1.0: consume 1 token → allow
  3. Else: reject, return wait_time = (1.0 - tokens) / refill_rate
```

## Public API

### `RateLimiter`

```rust
impl RateLimiter {
    pub fn new(config: &RateLimitConfig) -> Self
    pub fn check(&self, tool: &str) -> Result<(), Duration>
    pub fn available(&self, tool: &str) -> (f64, f64)
    pub fn register_tool(&self, name: &str, rps: f64, burst: u32)
}
```

- **`new`** — Creates buckets from config; one global bucket plus per-tool buckets.
- **`check`** — Returns `Ok(())` if allowed, `Err(wait_time)` if rate limited. Checks tool-specific bucket first, then global bucket. Both must have tokens.
- **`available`** — Returns `(global_available, tool_available)` tokens.
- **`register_tool`** — Dynamically register a new tool with custom rate limits.

### `RateLimitConfig`

```rust
pub struct RateLimitConfig {
    pub global_rps: f64,           // default: 10.0
    pub burst: u32,                // default: 20
    pub tools: HashMap<String, ToolRateLimit>,
}
```

### `ToolRateLimit`

```rust
pub struct ToolRateLimit {
    pub rps: f64,
    pub burst: u32,                // default: 5
}
```

## Configuration

Configured in `praxis.toml`:

```toml
[rate_limit]
global_rps = 10
burst = 20

[rate_limit.tools]
web_search = { rps = 2, burst = 5 }
code_exec = { rps = 1, burst = 3 }
```

### Programmatic

```rust
use praxis::rate_limit::{RateLimiter, RateLimitConfig};
use std::time::Duration;

let config = RateLimitConfig {
    global_rps: 10.0,
    burst: 20,
    tools: HashMap::new(),
};
let limiter = RateLimiter::new(&config);

// Check rate limit before executing
if let Err(wait) = limiter.check("web_fetch") {
    tokio::time::sleep(wait).await;
}
// Proceed with execution
```

## Dependencies

- **`std::sync::Mutex`** — Thread-safe bucket state.
- **`std::time::{Duration, Instant}`** — Token refill timing.

## Status

- ✅ Token bucket algorithm for global and per-tool limits
- ✅ Dynamic tool registration
- ✅ Wait time computation
- ✅ Thread-safe with Mutex
- ✅ Config-driven via `praxis.toml`
- ✅ Comprehensive test coverage

## Source

`src/rate_limit.rs`