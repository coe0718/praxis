use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

const FILENAME: &str = "oauth_tokens.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub provider: String,
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub token_type: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    /// None means the token does not expire (e.g. GitHub fine-grained PAT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub authorized_at: DateTime<Utc>,
}

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() >= exp)
            .unwrap_or(false)
    }

    /// True when the token will expire within 5 minutes.
    pub fn needs_refresh(&self) -> bool {
        self.expires_at
            .map(|exp| Utc::now() + Duration::minutes(5) >= exp)
            .unwrap_or(false)
    }
}

pub struct OAuthTokenStore {
    path: std::path::PathBuf,
}

impl OAuthTokenStore {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            path: data_dir.join(FILENAME),
        }
    }

    pub fn load(&self) -> Result<HashMap<String, OAuthToken>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("invalid oauth tokens in {}", self.path.display()))
    }

    pub fn get(&self, provider: &str) -> Result<Option<OAuthToken>> {
        Ok(self.load()?.remove(provider))
    }

    pub fn save(&self, token: &OAuthToken) -> Result<()> {
        let mut tokens = self.load().unwrap_or_default();
        tokens.insert(token.provider.clone(), token.clone());
        self.write(&tokens)
    }

    pub fn remove(&self, provider: &str) -> Result<bool> {
        let mut tokens = self.load()?;
        let removed = tokens.remove(provider).is_some();
        if removed {
            self.write(&tokens)?;
        }
        Ok(removed)
    }

    fn write(&self, tokens: &HashMap<String, OAuthToken>) -> Result<()> {
        let raw =
            serde_json::to_string_pretty(tokens).context("failed to serialize oauth tokens")?;
        fs::write(&self.path, raw)
            .with_context(|| format!("failed to write {}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trips_a_token() {
        let tmp = tempdir().unwrap();
        let store = OAuthTokenStore::new(tmp.path());

        let token = OAuthToken {
            provider: "github".to_string(),
            access_token: "ghp_test123".to_string(),
            refresh_token: None,
            token_type: "bearer".to_string(),
            scopes: vec!["repo".to_string(), "user".to_string()],
            expires_at: None,
            authorized_at: Utc::now(),
        };

        store.save(&token).unwrap();
        let loaded = store.get("github").unwrap().unwrap();
        assert_eq!(loaded.access_token, "ghp_test123");
        assert!(!loaded.is_expired());
    }

    #[test]
    fn remove_returns_false_when_absent() {
        let tmp = tempdir().unwrap();
        let store = OAuthTokenStore::new(tmp.path());
        assert!(!store.remove("github").unwrap());
    }

    #[test]
    fn expired_token_is_detected() {
        let token = OAuthToken {
            provider: "google".to_string(),
            access_token: "ya29.test".to_string(),
            refresh_token: Some("1//test".to_string()),
            token_type: "bearer".to_string(),
            scopes: vec![],
            expires_at: Some(Utc::now() - Duration::hours(1)),
            authorized_at: Utc::now(),
        };
        assert!(token.is_expired());
        assert!(token.needs_refresh());
    }
}
