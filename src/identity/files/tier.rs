//! Tiered identity file policy.
//!
//! Files are classified into three tiers:
//!
//! - **Free** — agent can write freely (e.g. JOURNAL.md, METRICS.md, PATTERNS.md).
//! - **Approval** — agent can write but the change should be reviewable
//!   (e.g. IDENTITY.md, AGENTS.md, GOALS.md).
//! - **Locked** — agent must never write; operator-only (e.g. SOUL.md, praxis.toml, .env).

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::paths::PraxisPaths;

/// Write permission tier for an identity file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTier {
    /// Agent can write freely without approval.
    Free,
    /// Agent can write but the change should be flagged for review.
    Approval,
    /// Agent must never write; operator-only.
    Locked,
}

/// Result of a write permission check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WritePermission {
    /// Write is allowed.
    Allowed,
    /// Write requires operator approval first.
    RequiresApproval,
    /// Write is forbidden.
    Denied(String),
}

#[derive(Default)]
/// Report from identity drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Whether any locked files have been modified since initialization.
    pub locked_files_modified: bool,
    /// Paths of locked files that have been modified.
    pub modified_locked_files: Vec<String>,
    /// SHA-256 hash of the current SOUL.md.
    pub soul_hash: String,
    /// SHA-256 hash of the SOUL.md at initialization (if recorded).
    pub original_soul_hash: Option<String>,
    /// Whether SOUL.md has changed since initialization.
    pub soul_drifted: bool,
}

impl DriftReport {
    /// Whether any drift was detected.
    pub fn has_drift(&self) -> bool {
        self.locked_files_modified || self.soul_drifted
    }
}

/// The file tier policy: maps file paths (relative to data_dir) to tiers.
#[derive(Debug, Clone)]
pub struct FileTierPolicy {
    tiers: Vec<(String, FileTier)>,
}

impl Default for FileTierPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTierPolicy {
    /// Create the default tier policy.
    pub fn new() -> Self {
        let tiers = vec![
            // Locked files — operator-only, never agent-writable.
            ("SOUL.md".to_string(), FileTier::Locked),
            ("praxis.toml".to_string(), FileTier::Locked),
            (".env".to_string(), FileTier::Locked),
            ("security.toml".to_string(), FileTier::Locked),
            ("vault.toml".to_string(), FileTier::Locked),
            ("master.key".to_string(), FileTier::Locked),
            // Approval files — agent can write but changes should be reviewable.
            ("IDENTITY.md".to_string(), FileTier::Approval),
            ("AGENTS.md".to_string(), FileTier::Approval),
            ("GOALS.md".to_string(), FileTier::Approval),
            ("ROADMAP.md".to_string(), FileTier::Approval),
            ("PROPOSALS.md".to_string(), FileTier::Approval),
            ("CAPABILITIES.md".to_string(), FileTier::Approval),
            ("operator_model.json".to_string(), FileTier::Approval),
            // Free files — agent can write freely.
            ("JOURNAL.md".to_string(), FileTier::Free),
            ("METRICS.md".to_string(), FileTier::Free),
            ("PATTERNS.md".to_string(), FileTier::Free),
            ("LEARNINGS.md".to_string(), FileTier::Free),
            ("DAY_COUNT".to_string(), FileTier::Free),
        ];
        Self { tiers }
    }

    /// Check whether writing to the given path (relative to data_dir) is allowed.
    pub fn check_write_permission(&self, relative_path: &str) -> WritePermission {
        // Strip any leading "./" or "/" for matching.
        let path = relative_path.trim_start_matches("./").trim_start_matches('/');

        // Find the most specific matching tier.
        let mut matched_tier = None;
        let mut matched_len = 0;
        for (pattern, tier) in &self.tiers {
            if path == pattern || path.ends_with(pattern) {
                let pattern_len = pattern.len();
                if pattern_len >= matched_len {
                    matched_tier = Some(*tier);
                    matched_len = pattern_len;
                }
            }
        }

        match matched_tier {
            Some(FileTier::Free) => WritePermission::Allowed,
            Some(FileTier::Approval) => WritePermission::RequiresApproval,
            Some(FileTier::Locked) => WritePermission::Denied(format!(
                "file '{path}' is locked (operator-only). Use a proposal to request changes."
            )),
            None => {
                // Unknown files default to Free (new files the agent creates).
                WritePermission::Allowed
            }
        }
    }

    /// Returns the tier for a given path, if known.
    pub fn tier_for(&self, relative_path: &str) -> Option<FileTier> {
        let path = relative_path.trim_start_matches("./").trim_start_matches('/');
        let mut matched_tier = None;
        let mut matched_len = 0;
        for (pattern, tier) in &self.tiers {
            if path == pattern || path.ends_with(pattern) {
                let pattern_len = pattern.len();
                if pattern_len >= matched_len {
                    matched_tier = Some(*tier);
                    matched_len = pattern_len;
                }
            }
        }
        matched_tier
    }

    /// Returns all locked file paths.
    pub fn locked_files(&self) -> HashSet<&str> {
        self.tiers
            .iter()
            .filter(|(_, tier)| *tier == FileTier::Locked)
            .map(|(path, _)| path.as_str())
            .collect()
    }
}

/// Compute the SHA-256 hash of a file's contents.
pub fn hash_file(path: &Path) -> Result<String> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let hash = Sha256::digest(contents.as_bytes());
    Ok(format!("{hash:x}"))
}

/// Detect identity drift: check if locked files have been modified.
pub fn detect_identity_drift(paths: &PraxisPaths) -> Result<DriftReport> {
    let policy = FileTierPolicy::new();
    let mut report = DriftReport::default();

    // Check all locked files for modifications.
    for locked_path in policy.locked_files() {
        let full_path = paths.data_dir.join(locked_path);
        if !full_path.exists() {
            continue;
        }
        // Compare against the original hash if recorded.
        let hash_path = paths.data_dir.join(format!("{locked_path}.orig_hash"));
        let current_hash = hash_file(&full_path)?;
        if hash_path.exists() {
            let original_hash = std::fs::read_to_string(&hash_path)
                .with_context(|| format!("failed to read {}", hash_path.display()))?;
            let original_hash = original_hash.trim();
            if current_hash != original_hash {
                report.locked_files_modified = true;
                report.modified_locked_files.push(locked_path.to_string());
            }
        }
    }

    // Specifically track SOUL.md drift.
    if paths.soul_file.exists() {
        report.soul_hash = hash_file(&paths.soul_file)?;
        let soul_hash_path = paths.data_dir.join("SOUL.md.orig_hash");
        if soul_hash_path.exists() {
            let original = std::fs::read_to_string(&soul_hash_path)
                .with_context(|| format!("failed to read {}", soul_hash_path.display()))?;
            let original = original.trim().to_string();
            report.original_soul_hash = Some(original.clone());
            report.soul_drifted = report.soul_hash != original;
        }
    }

    Ok(report)
}

/// Record the original hash of a file for future drift detection.
pub fn record_original_hash(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let hash = hash_file(path)?;
    let hash_path = path.with_extension(format!(
        "{}.orig_hash",
        path.extension().and_then(|e| e.to_str()).unwrap_or("")
    ));
    std::fs::write(&hash_path, format!("{hash}\n"))
        .with_context(|| format!("failed to write {}", hash_path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soul_is_locked() {
        let policy = FileTierPolicy::new();
        assert_eq!(
            policy.check_write_permission("SOUL.md"),
            WritePermission::Denied(
                "file 'SOUL.md' is locked (operator-only). Use a proposal to request changes."
                    .to_string()
            )
        );
    }

    #[test]
    fn journal_is_free() {
        let policy = FileTierPolicy::new();
        assert_eq!(policy.check_write_permission("JOURNAL.md"), WritePermission::Allowed);
    }

    #[test]
    fn identity_requires_approval() {
        let policy = FileTierPolicy::new();
        assert_eq!(policy.check_write_permission("IDENTITY.md"), WritePermission::RequiresApproval);
    }

    #[test]
    fn unknown_files_default_to_free() {
        let policy = FileTierPolicy::new();
        assert_eq!(policy.check_write_permission("my_new_file.md"), WritePermission::Allowed);
    }

    #[test]
    fn praxis_toml_is_locked() {
        let policy = FileTierPolicy::new();
        match policy.check_write_permission("praxis.toml") {
            WritePermission::Denied(_) => {}
            other => panic!("expected Denied, got {other:?}"),
        }
    }
}
