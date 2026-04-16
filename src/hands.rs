//! Hand manifests — installable role bundles that activate a coherent set of
//! tools, skills, and scheduling overrides in a single step.
//!
//! A *hand* is declared as a TOML file inside `hands/`.  When a hand is
//! activated it becomes the agent's current operational role; multiple hands
//! can be installed at once but only one is considered *active* at a time.
//!
//! ## Manifest format (`hands/<name>.toml`)
//!
//! ```toml
//! name        = "reviewer"
//! description = "Code-review assistant: reads PRs, leaves structured comments."
//! version     = "1.0.0"
//!
//! [tools]
//! required = ["read_file", "web_search"]
//! optional = ["write_file"]
//!
//! [skills]
//! # Names of skill files (without .md) that should be loaded when this hand is active.
//! load = ["review_pr", "summarise"]
//!
//! [schedule]
//! # Override quiet-hours for this hand (empty = inherit global schedule).
//! quiet_hours = [0, 1, 2, 3, 4, 5]
//!
//! [metadata]
//! author = "operator"
//! tags   = ["code", "review"]
//! ```

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Manifest types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandTools {
    /// Tool names that must be available for this hand to function.
    #[serde(default)]
    pub required: Vec<String>,
    /// Tool names that are used when available but not strictly required.
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandSkills {
    /// Skill file base-names (no `.md`) to activate with this hand.
    #[serde(default)]
    pub load: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandSchedule {
    /// Hours (0–23) that should be treated as quiet while this hand is active.
    /// Empty means inherit the global operator schedule.
    #[serde(default)]
    pub quiet_hours: Vec<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HandMetadata {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A fully parsed hand manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandManifest {
    /// Machine-readable identifier — matches the TOML filename without extension.
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub tools: HandTools,
    #[serde(default)]
    pub skills: HandSkills,
    #[serde(default)]
    pub schedule: HandSchedule,
    #[serde(default)]
    pub metadata: HandMetadata,
    /// Source path — set by the loader, not part of the serialised format.
    #[serde(skip)]
    pub source_path: PathBuf,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl HandManifest {
    /// Load a single manifest from `path`.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read hand manifest {}", path.display()))?;
        let mut manifest: Self = toml::from_str(&raw)
            .with_context(|| format!("invalid TOML in hand manifest {}", path.display()))?;
        manifest.source_path = path.to_path_buf();
        Ok(manifest)
    }

    /// One-line human-readable summary.
    pub fn summary(&self) -> String {
        format!(
            "{} v{} — {} (tools: {}, skills: {})",
            self.name,
            self.version,
            self.description,
            self.tools.required.len() + self.tools.optional.len(),
            self.skills.load.len(),
        )
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// All installed hands discovered from the `hands/` directory.
#[derive(Debug, Clone, Default)]
pub struct HandStore {
    pub hands: Vec<HandManifest>,
}

impl HandStore {
    /// Scan `hands_dir` and parse every `*.toml` file found.
    pub fn load(hands_dir: &Path) -> Result<Self> {
        if !hands_dir.is_dir() {
            return Ok(Self::default());
        }
        let mut hands = Vec::new();
        for entry in fs::read_dir(hands_dir)
            .with_context(|| format!("failed to read hands dir {}", hands_dir.display()))?
            .flatten()
        {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
                match HandManifest::load(&path) {
                    Ok(manifest) => hands.push(manifest),
                    Err(e) => log::warn!("skipping invalid hand manifest {}: {e}", path.display()),
                }
            }
        }
        hands.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self { hands })
    }

    /// Find a hand by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&HandManifest> {
        self.hands
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
    }

    /// Multi-line summary of all installed hands.
    pub fn summary(&self) -> String {
        if self.hands.is_empty() {
            return "hands: none installed".to_string();
        }
        let mut lines = vec![format!("hands: {} installed", self.hands.len())];
        for hand in &self.hands {
            lines.push(format!("  {}", hand.summary()));
        }
        lines.join("\n")
    }
}

/// Install a new hand manifest by writing a TOML file into `hands_dir`.
pub fn install_hand(paths: &PraxisPaths, manifest: &HandManifest) -> Result<PathBuf> {
    fs::create_dir_all(&paths.hands_dir)
        .with_context(|| format!("failed to create {}", paths.hands_dir.display()))?;
    let dest = paths.hands_dir.join(format!("{}.toml", manifest.name));
    let raw = toml::to_string_pretty(manifest).context("failed to serialise hand manifest")?;
    fs::write(&dest, raw)
        .with_context(|| format!("failed to write hand manifest {}", dest.display()))?;
    Ok(dest)
}

/// Remove an installed hand by name.  Returns `true` if the file existed.
pub fn remove_hand(paths: &PraxisPaths, name: &str) -> Result<bool> {
    let path = paths.hands_dir.join(format!("{name}.toml"));
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)
        .with_context(|| format!("failed to remove hand manifest {}", path.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::io::Write as IoWrite;

    use tempfile::TempDir;

    use super::*;

    fn write_manifest(dir: &Path, name: &str, toml: &str) {
        let mut f = fs::File::create(dir.join(format!("{name}.toml"))).unwrap();
        f.write_all(toml.as_bytes()).unwrap();
    }

    #[test]
    fn load_valid_manifest() {
        let dir = TempDir::new().unwrap();
        write_manifest(
            dir.path(),
            "reviewer",
            r#"
name        = "reviewer"
description = "Code review assistant"
version     = "2.0.0"

[tools]
required = ["read_file"]

[skills]
load = ["review_pr"]
"#,
        );
        let store = HandStore::load(dir.path()).unwrap();
        assert_eq!(store.hands.len(), 1);
        let hand = store.get("reviewer").unwrap();
        assert_eq!(hand.version, "2.0.0");
        assert_eq!(hand.tools.required, vec!["read_file"]);
    }

    #[test]
    fn empty_dir_returns_default() {
        let dir = TempDir::new().unwrap();
        let store = HandStore::load(dir.path()).unwrap();
        assert!(store.hands.is_empty());
    }

    #[test]
    fn summary_reflects_count() {
        let dir = TempDir::new().unwrap();
        write_manifest(
            dir.path(),
            "alpha",
            r#"name = "alpha"
description = "First"
"#,
        );
        let store = HandStore::load(dir.path()).unwrap();
        assert!(store.summary().contains("1 installed"));
    }
}
