//! Merkle hash-chain audit trail.
//!
//! Cryptographically links all actions so tampering with any past record
//! invalidates the chain.  Each entry hashes the previous hash + the current
//! payload.  The latest root hash can be compared across backups to detect
//! unauthorised modification.

use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::Path,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One entry in the audit chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: u64,
    pub timestamp: String,
    pub action: String,
    pub payload_json: String,
    pub prev_hash: String,
    pub hash: String,
}

/// Append-only audit log backed by a JSONL file.
#[derive(Debug, Clone)]
pub struct MerkleTrail {
    path: std::path::PathBuf,
}

impl MerkleTrail {
    pub fn new(path: &Path) -> Self {
        Self { path: path.to_path_buf() }
    }

    /// Append a new entry and return its hash.
    pub fn append(&self, action: &str, payload: &serde_json::Value) -> Result<String> {
        let entries = self.load()?;
        let prev_hash = entries.last().map(|e| e.hash.clone()).unwrap_or_default();
        let seq = entries.len() as u64 + 1;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let payload_json = payload.to_string();

        let hash = compute_hash(seq, &timestamp, action, &payload_json, &prev_hash);

        let entry = AuditEntry {
            seq,
            timestamp,
            action: action.to_string(),
            payload_json,
            prev_hash,
            hash: hash.clone(),
        };

        let line = serde_json::to_string(&entry).context("serialize audit entry")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("open audit trail {}", self.path.display()))?;
        writeln!(file, "{line}")
            .with_context(|| format!("write audit trail {}", self.path.display()))?;
        Ok(hash)
    }

    /// Load all entries from disk.
    pub fn load(&self) -> Result<Vec<AuditEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&self.path)
            .with_context(|| format!("read audit trail {}", self.path.display()))?;
        let mut out = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditEntry>(line) {
                Ok(e) => out.push(e),
                Err(e) => log::warn!("skipping corrupt audit line: {e}"),
            }
        }
        Ok(out)
    }

    /// Verify chain integrity: every entry's prev_hash matches the previous
    /// entry's hash, and every entry's hash recomputes correctly.
    pub fn verify(&self) -> Result<bool> {
        let entries = self.load()?;
        let mut prev_hash = String::new();
        for e in &entries {
            let expected =
                compute_hash(e.seq, &e.timestamp, &e.action, &e.payload_json, &e.prev_hash);
            if e.hash != expected {
                log::error!("audit chain broken at seq {}: hash mismatch", e.seq);
                return Ok(false);
            }
            if e.prev_hash != prev_hash {
                log::error!("audit chain broken at seq {}: prev_hash mismatch", e.seq);
                return Ok(false);
            }
            prev_hash = e.hash.clone();
        }
        Ok(true)
    }

    /// Return the latest (most recent) hash, if any.
    pub fn latest_hash(&self) -> Result<Option<String>> {
        let entries = self.load()?;
        Ok(entries.last().map(|e| e.hash.clone()))
    }
}

fn compute_hash(seq: u64, timestamp: &str, action: &str, payload: &str, prev: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_le_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(payload.as_bytes());
    hasher.update(prev.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn append_and_verify() {
        let dir = tempdir().unwrap();
        let trail = MerkleTrail::new(&dir.path().join("audit.jsonl"));
        let h1 = trail.append("test", &serde_json::json!({"x":1})).unwrap();
        let h2 = trail.append("test", &serde_json::json!({"x":2})).unwrap();
        assert_ne!(h1, h2);
        assert!(trail.verify().unwrap());
    }

    #[test]
    fn detect_tampering() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");
        let trail = MerkleTrail::new(&path);
        trail.append("test", &serde_json::json!({"x":1})).unwrap();
        trail.append("test", &serde_json::json!({"x":2})).unwrap();

        // Tamper with file: corrupt the first entry's hash field.
        let raw = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        let first = lines[0].replace("\"hash\":\"", "\"hash\":\"CORRUPTED");
        let tampered = format!("{first}\n{}", lines[1..].join("\n"));
        std::fs::write(&path, tampered).unwrap();

        assert!(!trail.verify().unwrap());
    }
}
