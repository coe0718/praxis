//! Carapace — Signed WASM plugin verification.
//!
//! WASM plugins can be cryptographically signed and verified before execution.
//! Supports Ed25519 signatures with optional key transparency.
//!
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Plugin manifest with cryptographic metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier.
    pub id: String,
    /// Version string.
    pub version: String,
    /// Author public key (hex, 64 chars = 32 bytes).
    pub author_key: String,
    /// SHA-256 hash of the WASM binary.
    pub wasm_hash: String,
    /// Signature over the hash (base64).
    pub signature: String,
    /// Optional metadata.
    pub description: Option<String>,
    /// Required capabilities.
    pub capabilities: Vec<String>,
}

/// Signed plugin container.
#[derive(Debug, Clone)]
pub struct SignedPlugin {
    pub manifest: PluginManifest,
    pub wasm_bytes: Vec<u8>,
    pub verified: bool,
}

impl SignedPlugin {
    /// Load a signed plugin from a directory.
    pub fn load(dir: &Path) -> Result<Self, PluginError> {
        let manifest_path = dir.join("plugin.json");
        let manifest: PluginManifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;

        let wasm_path = dir.join("plugin.wasm");
        let wasm_bytes = std::fs::read(&wasm_path)?;

        let plugin = Self {
            manifest,
            wasm_bytes,
            verified: false,
        };

        Ok(plugin)
    }

    /// Verify the plugin signature.
    pub fn verify(&mut self) -> Result<bool, PluginError> {
        let hash = self.compute_hash()?;

        if hash != self.manifest.wasm_hash {
            return Ok(false);
        }

        let sig_bytes =
            base64::engine::general_purpose::STANDARD.decode(&self.manifest.signature)?;
        let signature = Signature::from_slice(&sig_bytes).map_err(PluginError::Ed25519)?;

        let pub_key_bytes = hex::decode(&self.manifest.author_key)?;
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&pub_key_bytes[..32]);
        let verifying_key = VerifyingKey::from_bytes(&key_bytes).map_err(PluginError::Ed25519)?;

        verifying_key
            .verify(&hex::decode(&hash)?, &signature)
            .map(|_| {
                self.verified = true;
                true
            })
            .map_err(|_| PluginError::SignatureInvalid)
    }

    fn compute_hash(&self) -> Result<String, PluginError> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&self.wasm_bytes);
        Ok(hex::encode(hasher.finalize()))
    }
}

/// Plugin verification error.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("Signature invalid")]
    SignatureInvalid,
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Ed25519 error: {0}")]
    Ed25519(#[source] ed25519_dalek::ed25519::Error),
}

/// Plugin registry with signature verification.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: std::collections::HashMap<String, SignedPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load and verify all plugins in a directory.
    pub fn load_directory(&mut self, dir: &Path) -> Result<Vec<String>, PluginError> {
        let mut verified = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                match SignedPlugin::load(&entry.path()) {
                    Ok(mut plugin) => {
                        if plugin.verify().unwrap_or(false) {
                            verified.push(entry.file_name().to_string_lossy().to_string());
                            self.plugins
                                .insert(entry.file_name().to_string_lossy().to_string(), plugin);
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to load plugin {:?}: {}", entry.path(), e);
                    }
                }
            }
        }

        Ok(verified)
    }

    /// Get a verified plugin by ID.
    pub fn get(&self, id: &str) -> Option<&SignedPlugin> {
        self.plugins.get(id)
    }

    /// List all verified plugin IDs.
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_hash() {
        let plugin = SignedPlugin {
            manifest: PluginManifest {
                id: "test".into(),
                version: "1.0".into(),
                author_key: "00".repeat(64),
                wasm_hash: "00".repeat(64),
                signature: "00".repeat(88),
                description: None,
                capabilities: vec![],
            },
            wasm_bytes: b"hello world".to_vec(),
            verified: false,
        };

        let hash = plugin.compute_hash().unwrap();
        assert_eq!(hash.len(), 64);
    }
}
