//! Dynamic webhook subscriptions.
//!
//! Operators can register per-purpose webhook endpoints without code changes.
//! Each subscription defines a URL path, HMAC secret for signature verification,
//! and a list of allowed event types.  When an inbound request matches, the
//! dashboard verifies the HMAC-SHA256 signature and fires a `WakeIntent` with
//! the webhook payload embedded in the session task.
//!
//! Persisted as `webhooks.json` in the data directory.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Named webhook subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    /// Unique name — also becomes the URL path segment (`/webhook/{name}`).
    pub name: String,
    /// Human-readable label for listing.
    pub description: String,
    /// HMAC-SHA256 signing secret for `X-Signature-256` header validation.
    /// Empty = no signature required (insecure — only for trusted networks).
    pub secret: Option<String>,
    /// Comma-separated event types this webhook accepts (e.g. "push,deploy").
    /// If empty, all events are accepted.
    pub events: String,
    /// When this subscription was created.
    pub created_at: DateTime<Utc>,
    /// When this subscription last received a valid request.
    pub last_triggered_at: Option<DateTime<Utc>>,
    /// Total number of verified requests received.
    pub trigger_count: u64,
    /// When true, forward the webhook payload directly to the messaging bus
    /// without going through the agent/LLM loop. Useful for notifications,
    /// alerts, and CI/CD status updates that don't need agent reasoning.
    #[serde(default)]
    pub direct_delivery: bool,
}

/// Persistent registry of webhook subscriptions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookStore {
    pub webhooks: Vec<Webhook>,
}

impl WebhookStore {
    /// Load from disk, returning an empty store if the file is absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    /// Persist to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize webhook store")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Find a webhook by name.
    pub fn get(&self, name: &str) -> Option<&Webhook> {
        self.webhooks.iter().find(|w| w.name == name)
    }

    /// Add or replace a webhook by name.
    pub fn upsert(&mut self, webhook: Webhook) {
        self.webhooks.retain(|w| w.name != webhook.name);
        self.webhooks.push(webhook);
    }

    /// Remove a webhook by name.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.webhooks.len();
        self.webhooks.retain(|w| w.name != name);
        self.webhooks.len() < before
    }
}

impl Webhook {
    /// Verify an HMAC-SHA256 signature against the request body.
    ///
    /// The client sends `X-Signature-256: sha256=<hex>` and
    /// `X-Webhook-Timestamp: <unix-seconds>`.
    ///
    /// We compute `HMAC-SHA256(timestamp || ":" || body, secret)` and compare.
    /// Requests older than 5 minutes are rejected (anti-replay).
    pub fn verify_signature(&self, timestamp: &str, body: &[u8], signature: &str) -> Result<bool> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = match &self.secret {
            Some(s) => s.as_bytes(),
            None => return Ok(true), // No secret → always passes.
        };

        // Anti-replay: reject timestamps older than 5 minutes.
        let ts: i64 =
            timestamp.parse().context("invalid X-Webhook-Timestamp: must be unix seconds")?;
        let request_time =
            DateTime::from_timestamp(ts, 0).with_context(|| format!("invalid timestamp: {ts}"))?;
        let age = Utc::now().signed_duration_since(request_time);
        if age.num_seconds().abs() > 300 {
            return Ok(false); // Too old or in the future.
        }

        // Parse signature: "sha256=<hex>"
        let expected_hex = signature
            .strip_prefix("sha256=")
            .context("signature must be prefixed with 'sha256='")?;

        let expected = hex::decode(expected_hex).context("invalid hex in signature")?;

        let mut mac: Hmac<Sha256> =
            Hmac::new_from_slice(secret).context("HMAC initialization failed")?;
        mac.update(timestamp.as_bytes());
        mac.update(b":");
        mac.update(body);

        Ok(mac.verify_slice(&expected).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_store() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("webhooks.json");

        let mut store = WebhookStore::default();
        store.upsert(Webhook {
            name: "ci-cd".into(),
            description: "CI/CD pipeline".into(),
            secret: Some("secret123".into()),
            events: "deploy,build".into(),
            created_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            direct_delivery: false,
        });
        store.save(&path).unwrap();

        let loaded = WebhookStore::load(&path).unwrap();
        assert_eq!(loaded.webhooks.len(), 1);
        assert_eq!(loaded.webhooks[0].name, "ci-cd");
    }

    #[test]
    fn empty_store_loads_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WebhookStore::load(&tmp.path().join("nonexistent.json")).unwrap();
        assert!(store.webhooks.is_empty());
    }

    #[test]
    fn no_secret_always_passes() {
        let wh = Webhook {
            name: "open".into(),
            description: "no secret".into(),
            secret: None,
            events: "".into(),
            created_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            direct_delivery: false,
        };
        assert!(wh.verify_signature("1234567890", b"hello", "anything").unwrap());
    }

    #[test]
    fn valid_signature_passes() {
        let wh = Webhook {
            name: "test".into(),
            description: "test".into(),
            secret: Some("my-secret-key".into()),
            events: "".into(),
            created_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            direct_delivery: false,
        };

        let timestamp = Utc::now().timestamp().to_string();
        let body = b"{\"event\":\"push\"}";

        // Compute valid signature.
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"my-secret-key").unwrap();
        mac.update(timestamp.as_bytes());
        mac.update(b":");
        mac.update(body);
        let sig = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(wh.verify_signature(&timestamp, body, &sig).unwrap());
    }

    #[test]
    fn old_timestamp_rejected() {
        let wh = Webhook {
            name: "test".into(),
            description: "test".into(),
            secret: Some("secret".into()),
            events: "".into(),
            created_at: Utc::now(),
            last_triggered_at: None,
            trigger_count: 0,
            direct_delivery: false,
        };

        // Timestamp 10 minutes in the past.
        let old = (Utc::now().timestamp() - 600).to_string();
        assert!(!wh.verify_signature(&old, b"body", "sha256=deadbeef").unwrap());
    }
}
