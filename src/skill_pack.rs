//! Skill packs — bundled collections with manifest.
//!
//! Skills can be packaged as reusable collections with
//! standardized manifests and dependency resolution.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Manifest for a skill pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPackManifest {
    /// Pack identifier.
    pub id: String,
    /// Pack version.
    pub version: String,
    /// Pack description.
    pub description: String,
    /// Author information.
    pub author: String,
    /// List of included skills.
    pub skills: Vec<SkillInfo>,
    /// Dependencies on other packs.
    pub dependencies: Vec<String>,
    /// Compatibility tags.
    pub tags: Vec<String>,
}

/// Information about a single skill in a pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    /// Skill identifier.
    pub id: String,
    /// Entry point file.
    pub entry: String,
    /// Required capabilities.
    pub requires: Vec<String>,
    /// Configuration schema.
    pub config_schema: Option<serde_json::Value>,
}

/// A skill pack.
#[derive(Debug, Clone)]
pub struct SkillPack {
    pub manifest: SkillPackManifest,
    pub path: PathBuf,
}

impl SkillPack {
    /// Load a skill pack from a directory.
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error> {
        let manifest_path = path.join("pack.toml");
        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: SkillPackManifest = toml::from_str(&content)?;

        Ok(Self { manifest, path })
    }

    /// Install the pack to the skills directory.
    pub fn install(&self, target_dir: &std::path::Path) -> Result<(), anyhow::Error> {
        let dest = target_dir.join(&self.manifest.id);
        std::fs::create_dir_all(&dest)?;

        for skill in &self.manifest.skills {
            let src = self.path.join(&skill.entry);
            let dst = dest.join(&skill.entry);
            if src.exists() {
                std::fs::copy(&src, &dst)?;
            }
        }

        Ok(())
    }

    /// Validate dependencies.
    pub fn validate_dependencies(&self, installed: &[&str]) -> Vec<String> {
        self.manifest.dependencies
            .iter()
            .filter(|dep| !installed.contains(&dep.as_str()))
            .cloned()
            .collect()
    }
}

/// Registry of installed skill packs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkillPackRegistry {
    pub packs: std::collections::HashMap<String, SkillPackManifest>,
}

impl SkillPackRegistry {
    /// Load registry from disk.
    pub fn load(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Save registry to disk.
    pub fn save(&self, path: &std::path::Path) -> Result<(), anyhow::Error> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a pack to the registry.
    pub fn add(&mut self, manifest: SkillPackManifest) {
        self.packs.insert(manifest.id.clone(), manifest);
    }

    /// Get installed pack IDs.
    pub fn installed_ids(&self) -> Vec<&str> {
        self.packs.keys().map(|s| s.as_str()).collect()
    }
}