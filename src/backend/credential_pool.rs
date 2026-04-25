//! Credential pooling — round-robin rotation across multiple API keys per provider.
//!
//! When `features.credential_pooling = true` in `praxis.toml`, the daemon reads
//! additional keys from `<UPPER_PROVIDER>_API_KEY_<N>` env vars (1-indexed) and
//! rotates through them.  Keys that hit a 429 are temporarily cooled down for a
//! configurable period.
//!
//! Example `.env`:
//! ```ignore
//! OPENAI_API_KEY=sk-primary
//! OPENAI_API_KEY_2=sk-backup-1
//! OPENAI_API_KEY_3=sk-backup-2
//! ```

use std::{
    collections::HashMap,
    sync::OnceLock,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

/// Global registry of credential pools, keyed by provider name.
/// Populated once at daemon startup when `features.credential_pooling` is enabled.
pub static CREDENTIAL_POOLS: OnceLock<HashMap<String, CredentialPool>> = OnceLock::new();

/// Initialise the global pool registry from environment variables for the
/// given providers.  Only providers that have ≥2 keys get a pool entry.
pub fn init_pools(provider_names: &[&str]) {
    let mut pools = HashMap::new();
    for provider in provider_names {
        if let Some(pool) = CredentialPool::from_env(provider) {
            log::info!(
                "credential_pool: {} keys registered for provider '{}'",
                pool.len(),
                provider
            );
            pools.insert((*provider).to_string(), pool);
        }
    }
    if !pools.is_empty() {
        let _ = CREDENTIAL_POOLS.set(pools);
    }
}

/// Minimum number of keys to justify pooling overhead.
const MIN_POOL_SIZE: usize = 2;

/// Default cooldown after a 429 response before retrying a key.
const DEFAULT_COOLDOWN_SECS: u64 = 30;

/// A rotating pool of API keys for a single provider.
pub struct CredentialPool {
    keys: Vec<String>,
    /// Round-robin index — wraps around with modulo.
    index: AtomicUsize,
    /// Per-key cooldown deadline.  `Instant` is monotonic and safe to compare
    /// without worrying about clock jumps.
    cooldowns: std::sync::Mutex<Vec<Option<Instant>>>,
    cooldown_duration: Duration,
}

impl CredentialPool {
    /// Build a pool by reading `<PROVIDER>_API_KEY` + `<PROVIDER>_API_KEY_<N>`
    /// from the environment.  Returns `None` if fewer than 2 keys are found
    /// (pooling is not useful for a single key).
    pub fn from_env(provider: &str) -> Option<Self> {
        let env_prefix = format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"));

        let mut keys = Vec::new();

        // Primary key (no suffix).
        if let Ok(key) = std::env::var(&env_prefix) {
            keys.push(key);
        }

        // Numbered keys: _2, _3, ...
        for n in 2..=20u32 {
            if let Ok(key) = std::env::var(format!("{env_prefix}_{n}")) {
                keys.push(key);
            } else {
                break;
            }
        }

        if keys.len() < MIN_POOL_SIZE {
            return None;
        }

        let len = keys.len();
        Some(Self {
            keys,
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![None; len]),
            cooldown_duration: Duration::from_secs(DEFAULT_COOLDOWN_SECS),
        })
    }

    /// Pick the next available (non-cooled-down) key using round-robin.
    /// Falls back to the primary key if all are on cooldown.
    pub fn next_key(&self) -> &str {
        let cooldowns = self.cooldowns.lock().expect("credential pool mutex poisoned");
        let len = self.keys.len();
        let start = self.index.fetch_add(1, Ordering::Relaxed) % len;

        // Scan from `start` forward, wrapping, looking for a non-cooled key.
        for offset in 0..len {
            let i = (start + offset) % len;
            let cooled = cooldowns[i].is_some_and(|deadline| deadline > Instant::now());
            if !cooled {
                return &self.keys[i];
            }
        }

        // All on cooldown — use primary as a last resort.
        &self.keys[0]
    }

    /// Mark the key that was just used as rate-limited (429).
    /// Returns `true` if the key was found in the pool.
    pub fn mark_rate_limited(&self, key: &str) -> bool {
        let mut cooldowns = self.cooldowns.lock().expect("credential pool mutex poisoned");
        for (i, k) in self.keys.iter().enumerate() {
            if k == key {
                cooldowns[i] = Some(Instant::now() + self.cooldown_duration);
                log::warn!(
                    "credential_pool: key {} for provider rate-limited, cooling for {}s",
                    i + 1,
                    self.cooldown_duration.as_secs()
                );
                return true;
            }
        }
        false
    }

    /// Number of keys in the pool (including those on cooldown).
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether the pool is empty (should never be true if constructed via `from_env`).
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robins_through_keys() {
        // Construct directly to avoid env var dependency.
        let pool = CredentialPool {
            keys: vec!["key-a".to_string(), "key-b".to_string(), "key-c".to_string()],
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![None, None, None]),
            cooldown_duration: Duration::from_secs(30),
        };

        // Sequential calls should cycle.
        assert_eq!(pool.next_key(), "key-a");
        assert_eq!(pool.next_key(), "key-b");
        assert_eq!(pool.next_key(), "key-c");
        assert_eq!(pool.next_key(), "key-a"); // wraps
    }

    #[test]
    fn skips_cooled_down_keys() {
        let pool = CredentialPool {
            keys: vec!["key-a".to_string(), "key-b".to_string()],
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![
                Some(Instant::now() + Duration::from_secs(300)), // key-a cooled
                None,
            ]),
            cooldown_duration: Duration::from_secs(30),
        };

        // Should skip key-a and return key-b.
        assert_eq!(pool.next_key(), "key-b");
    }

    #[test]
    fn falls_back_to_primary_when_all_cooled() {
        let pool = CredentialPool {
            keys: vec!["key-a".to_string(), "key-b".to_string()],
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![
                Some(Instant::now() + Duration::from_secs(300)),
                Some(Instant::now() + Duration::from_secs(300)),
            ]),
            cooldown_duration: Duration::from_secs(30),
        };

        // All cooled — falls back to primary.
        assert_eq!(pool.next_key(), "key-a");
    }

    #[test]
    fn mark_rate_limited_sets_cooldown() {
        let pool = CredentialPool {
            keys: vec!["key-a".to_string(), "key-b".to_string()],
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![None, None]),
            cooldown_duration: Duration::from_secs(30),
        };

        assert!(pool.mark_rate_limited("key-b"));
        // key-b should now be on cooldown.
        let cooldowns = pool.cooldowns.lock().unwrap();
        assert!(cooldowns[1].is_some());
    }

    #[test]
    fn mark_rate_limited_returns_false_for_unknown_key() {
        let pool = CredentialPool {
            keys: vec!["key-a".to_string()],
            index: AtomicUsize::new(0),
            cooldowns: std::sync::Mutex::new(vec![None]),
            cooldown_duration: Duration::from_secs(30),
        };

        assert!(!pool.mark_rate_limited("unknown"));
    }
}
