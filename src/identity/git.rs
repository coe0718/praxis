//! Git-native agent identity and skills.
//!
//! Agents can be tracked in git repos with skills as files,
//! branch personalities, and version-controlled evolution.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Git-based agent identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitIdentity {
    /// Git repository path.
    pub repo_path: PathBuf,
    /// Current branch (personality variant).
    pub branch: String,
    /// Agent manifest (agent.toml).
    pub manifest: AgentManifest,
    /// Last commit hash.
    pub head_sha: Option<String>,
}

/// Agent manifest stored in git.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentManifest {
    /// Agent name.
    pub name: String,
    /// Agent version.
    pub version: String,
    /// Description.
    pub description: String,
    /// Capabilities/skills.
    pub capabilities: Vec<String>,
    /// Personality traits.
    pub personality: PersonalityTraits,
}

/// Personality traits that vary by branch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonalityTraits {
    /// Enthusiasm level (0-10).
    pub enthusiasm: u8,
    /// Formality level (0-10).
    pub formality: u8,
    /// Proactiveness (0-10).
    pub proactivity: u8,
    /// Risk tolerance (0-10).
    pub risk_tolerance: u8,
}

impl GitIdentity {
    /// Initialize a new git-based identity.
    pub fn init(repo_path: PathBuf) -> Result<Self, anyhow::Error> {
        std::fs::create_dir_all(&repo_path)?;

        let manifest = AgentManifest {
            name: "praxis-agent".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Praxis autonomous agent".to_string(),
            capabilities: vec![],
            personality: PersonalityTraits::default(),
        };

        let identity = Self {
            repo_path,
            branch: "main".to_string(),
            manifest,
            head_sha: None,
        };

        identity.save_manifest()?;
        Ok(identity)
    }

    /// Save manifest to git repo.
    pub fn save_manifest(&self) -> Result<(), anyhow::Error> {
        let manifest_path = self.repo_path.join("agent.toml");
        let content = toml::to_string_pretty(&self.manifest)?;
        std::fs::write(manifest_path, content)?;
        Ok(())
    }

    /// Load identity from existing git repo.
    pub fn load(repo_path: PathBuf) -> Result<Self, anyhow::Error> {
        let manifest_path = repo_path.join("agent.toml");
        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: AgentManifest = toml::from_str(&content)?;

        let head_sha = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&repo_path)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        Ok(Self {
            repo_path,
            branch: "main".to_string(),
            manifest,
            head_sha,
        })
    }

    /// Check for skill updates from git.
    pub fn pull_skills(&mut self) -> Result<Vec<String>, anyhow::Error> {
        let output = std::process::Command::new("git")
            .args(["pull", "origin", &self.branch])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            let skills = self.scan_for_new_skills()?;
            self.head_sha = Some(
                String::from_utf8_lossy(
                    &std::process::Command::new("git")
                        .args(["rev-parse", "HEAD"])
                        .current_dir(&self.repo_path)
                        .output()?
                        .stdout,
                )
                .trim()
                .to_string(),
            );
            Ok(skills)
        } else {
            Ok(vec![])
        }
    }

    /// Scan repo for skill files.
    pub fn scan_for_new_skills(&self) -> Result<Vec<String>, anyhow::Error> {
        let skills_dir = self.repo_path.join("skills");
        if !skills_dir.exists() {
            return Ok(vec![]);
        }

        let mut skills = vec![];
        for entry in std::fs::read_dir(skills_dir)? {
            let entry = entry?;
            if entry.path().extension().is_some_and(|e| e == "rs" || e == "toml")
                && let Some(name) = entry.file_name().to_str()
            {
                skills.push(name.to_string());
            }
        }
        Ok(skills)
    }

    /// Switch to a different personality branch.
    pub fn checkout_branch(&mut self, branch: &str) -> Result<(), anyhow::Error> {
        let output = std::process::Command::new("git")
            .args(["checkout", branch])
            .current_dir(&self.repo_path)
            .output()?;

        if output.status.success() {
            self.branch = branch.to_string();
            // Reload manifest for new personality
            let manifest_path = self.repo_path.join("agent.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)?;
                self.manifest = toml::from_str(&content)?;
            }
            Ok(())
        } else {
            anyhow::bail!("Failed to checkout branch: {}", branch)
        }
    }
}
