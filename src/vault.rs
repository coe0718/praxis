//! Credential vault proxy — resolves named secrets at request time from a
//! local TOML file (`vault.toml`), never storing raw values in the main config.
//!
//! The vault maps *alias names* to either a literal secret value or an
//! environment-variable reference.  Code throughout the OAuth/provider layer
//! calls `vault.resolve(name)` instead of reading environment variables
//! directly, keeping all secret access auditable from a single file.
//!
//! ## `vault.toml` format
//!
//! ```toml
//! # Literal value (only acceptable in dev/testing — flag a warning in prod)
//! [secrets.my_literal]
//! value = "sk-abc123"
//!
//! # Environment variable reference (preferred)
//! [secrets.anthropic_key]
//! env = "ANTHROPIC_API_KEY"
//!
//! # Environment variable with fallback literal
//! [secrets.openai_key]
//! env      = "OPENAI_API_KEY"
//! fallback = "sk-test-placeholder"
//! ```
//!
//! `vault.toml` should be listed in `.gitignore`.  Praxis will warn on startup
//! if the file contains any `value` entries (literals) and the instance is not
//! in development mode.

use std::{collections::HashMap, env, fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ── Secret entry ──────────────────────────────────────────────────────────────

/// A single named secret entry in the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VaultEntry {
    /// Literal value — convenient for dev, avoid in production.
    Literal { value: String },
    /// Resolve from an environment variable, with an optional fallback literal.
    EnvVar {
        env: String,
        #[serde(default)]
        fallback: Option<String>,
    },
}

impl VaultEntry {
    /// Resolve the entry to a concrete secret string.
    ///
    /// Returns `Ok(None)` when an env-var entry has no fallback and the
    /// variable is unset, letting callers decide whether to error.
    pub fn resolve(&self) -> Result<Option<String>> {
        match self {
            Self::Literal { value } => Ok(Some(value.clone())),
            Self::EnvVar { env: var, fallback } => match env::var(var) {
                Ok(v) if !v.is_empty() => Ok(Some(v)),
                _ => Ok(fallback.clone()),
            },
        }
    }

    /// True if this entry exposes a literal (rather than an env-var reference).
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal { .. })
    }
}

// ── Vault ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default)]
    pub secrets: HashMap<String, VaultEntry>,
}

impl Vault {
    /// Load the vault from `path`.  Returns an empty vault if the file does
    /// not exist.  Transparently decrypts if a `master.key` exists next to it.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let content = crate::crypto::maybe_decrypt(path, &raw)
                    .with_context(|| format!("failed to decrypt vault at {}", path.display()))?;
                toml::from_str(&content)
                    .with_context(|| format!("invalid vault at {}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read vault {}", path.display())),
        }
    }

    /// Persist the vault to `path`.  Transparently encrypts if a `master.key`
    /// exists next to it.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("failed to serialise vault")?;
        let content = crate::crypto::maybe_encrypt(path, &raw)
            .with_context(|| format!("failed to encrypt vault at {}", path.display()))?;
        fs::write(path, content)
            .with_context(|| format!("failed to write vault {}", path.display()))
    }

    /// Resolve a named secret.
    ///
    /// - Returns `Ok(value)` if the entry exists and resolves.
    /// - Returns `Err` if the entry is missing or resolution fails.
    pub fn resolve(&self, name: &str) -> Result<String> {
        let entry = self
            .secrets
            .get(name)
            .with_context(|| format!("vault: unknown secret '{name}'"))?;
        entry.resolve()?.with_context(|| {
            format!("vault: secret '{name}' resolved to nothing (env var unset and no fallback)")
        })
    }

    /// Resolve a named secret, returning `None` instead of an error when the
    /// secret is absent or unset.
    pub fn resolve_optional(&self, name: &str) -> Option<String> {
        self.secrets.get(name)?.resolve().ok().flatten()
    }

    /// Add or replace an entry.
    pub fn set(&mut self, name: impl Into<String>, entry: VaultEntry) {
        self.secrets.insert(name.into(), entry);
    }

    /// Remove a named entry.  Returns `true` if it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.secrets.remove(name).is_some()
    }

    /// Return all entries that are literals (useful for auditing).
    pub fn literal_entries(&self) -> Vec<&str> {
        self.secrets
            .iter()
            .filter(|(_, v)| v.is_literal())
            .map(|(k, _)| k.as_str())
            .collect()
    }

    /// Human-readable summary (never includes the secret values).
    pub fn summary(&self) -> String {
        if self.secrets.is_empty() {
            return "vault: empty".to_string();
        }
        let mut lines = vec![format!("vault: {} secret(s)", self.secrets.len())];
        let mut names: Vec<&str> = self.secrets.keys().map(String::as_str).collect();
        names.sort();
        for name in names {
            let kind = match &self.secrets[name] {
                VaultEntry::Literal { .. } => "literal ⚠",
                VaultEntry::EnvVar { env, .. } => {
                    // Borrow issue: just note it's an env-var type.
                    let _ = env;
                    "env-var"
                }
            };
            lines.push(format!("  {name} ({kind})"));
        }
        lines.join("\n")
    }
}

// ── Vault-aware resolution helpers ───────────────────────────────────────────

/// Resolve a secret by name from the vault, falling back to a direct
/// environment variable lookup with the same name (uppercased).
///
/// This lets existing code that reads `env::var("ANTHROPIC_API_KEY")` be
/// migrated incrementally: add an entry to the vault and the behaviour
/// becomes vault-controlled; callers that haven't migrated still work via env.
pub fn resolve_with_fallback(vault: &Vault, name: &str) -> Result<String> {
    if let Some(value) = vault.resolve_optional(name) {
        return Ok(value);
    }
    // Env-var fallback: uppercase the name, replacing hyphens with underscores.
    let env_key: String = name
        .chars()
        .map(|c| if c == '-' { '_' } else { c.to_ascii_uppercase() })
        .collect();
    env::var(&env_key).with_context(|| {
        format!("vault: secret '{name}' not in vault and env var '{env_key}' is unset")
    })
}

/// Validate that no vault entry has a literal value — useful at startup to
/// warn operators who have left dev secrets in place.
pub fn audit_literals(vault: &Vault) -> Vec<String> {
    vault
        .literal_entries()
        .into_iter()
        .map(|name| {
            format!("vault entry '{name}' uses a literal value — prefer env = \"VAR_NAME\"")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::env;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn literal_entry_resolves() {
        let mut vault = Vault::default();
        vault.set("test_key", VaultEntry::Literal { value: "secret123".to_string() });
        assert_eq!(vault.resolve("test_key").unwrap(), "secret123");
    }

    #[test]
    fn env_entry_resolves_from_env() {
        // Safety: test binary is single-threaded during env mutation.
        unsafe {
            env::set_var("PRAXIS_TEST_SECRET_7261", "from_env");
        }
        let mut vault = Vault::default();
        vault.set(
            "my_secret",
            VaultEntry::EnvVar {
                env: "PRAXIS_TEST_SECRET_7261".to_string(),
                fallback: None,
            },
        );
        assert_eq!(vault.resolve("my_secret").unwrap(), "from_env");
        unsafe {
            env::remove_var("PRAXIS_TEST_SECRET_7261");
        }
    }

    #[test]
    fn env_entry_uses_fallback_when_unset() {
        let mut vault = Vault::default();
        vault.set(
            "my_secret",
            VaultEntry::EnvVar {
                env: "PRAXIS_NONEXISTENT_VAR_99".to_string(),
                fallback: Some("fallback_val".to_string()),
            },
        );
        assert_eq!(vault.resolve("my_secret").unwrap(), "fallback_val");
    }

    #[test]
    fn missing_entry_returns_err() {
        let vault = Vault::default();
        assert!(vault.resolve("nonexistent").is_err());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("vault.toml");

        let mut vault = Vault::default();
        vault.set(
            "api_key",
            VaultEntry::EnvVar {
                env: "ANTHROPIC_API_KEY".to_string(),
                fallback: None,
            },
        );
        vault.save(&path).unwrap();

        let loaded = Vault::load(&path).unwrap();
        assert!(loaded.secrets.contains_key("api_key"));
    }

    #[test]
    fn audit_literals_reports_literal_entries() {
        let mut vault = Vault::default();
        vault.set("literal_key", VaultEntry::Literal { value: "oops".to_string() });
        let warnings = audit_literals(&vault);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("literal_key"));
    }
}
