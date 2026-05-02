//! Profile isolation — separate configurations, memories, and tools per profile.
//!
//! Each profile gets its own namespace within the data directory:
//!   `<data_dir>/profiles/<name>/`
//!
//! This allows running multiple agent personas from a single Praxis instance,
//! each with independent memory, tools, goals, and configuration overrides.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

/// A profile definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique profile name.
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: String,
    /// Optional config overrides (TOML key-value).
    #[serde(default = "default_toml_table")]
    pub config_overrides: toml::Value,
    /// Whether this profile is active.
    #[serde(default)]
    pub active: bool,
    /// When the profile was created.
    #[serde(default)]
    pub created_at: String,
}

/// Profiles collection stored in `profiles.toml`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profiles {
    pub profiles: Vec<Profile>,
    /// Name of the currently active profile.
    #[serde(default = "default_profile_name")]
    pub active: String,
}

fn default_profile_name() -> String {
    "default".to_string()
}

fn default_toml_table() -> toml::Value {
    toml::Value::Table(toml::map::Map::new())
}

impl Profiles {
    /// Load profiles from the profiles.toml file.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
    }

    /// Save profiles to the profiles.toml file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let raw = toml::to_string_pretty(self).context("serializing profiles")?;
        fs::write(path, raw).with_context(|| format!("writing {}", path.display()))
    }

    /// Get a profile by name.
    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Get a mutable reference to a profile by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.name == name)
    }

    /// Create a new profile.
    pub fn create(&mut self, name: &str, description: &str) -> Result<()> {
        if self.get(name).is_some() {
            bail!("profile '{}' already exists", name);
        }
        self.profiles.push(Profile {
            name: name.to_string(),
            description: description.to_string(),
            config_overrides: toml::Value::Table(toml::map::Map::new()),
            active: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        Ok(())
    }

    /// Switch to a profile, creating its isolated directory.
    pub fn switch(&mut self, name: &str) -> Result<PathBuf> {
        if self.get(name).is_none() && name != "default" {
            bail!(
                "profile '{}' does not exist. Create it first with `praxis profile create`.",
                name
            );
        }

        // Deactivate current
        if let Some(current) = self.get_mut(&self.active.clone()) {
            current.active = false;
        }

        // Activate new
        if let Some(profile) = self.get_mut(name) {
            profile.active = true;
        }
        self.active = name.to_string();

        Ok(profile_dir(name))
    }

    /// Remove a profile.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        if name == "default" {
            bail!("cannot remove the default profile");
        }
        if self.active == name {
            bail!("cannot remove the active profile — switch to another first");
        }
        let idx = self
            .profiles
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", name))?;
        self.profiles.remove(idx);
        Ok(())
    }
}

/// Get the isolated directory for a profile.
pub fn profile_dir(name: &str) -> PathBuf {
    // Profile directories live under <data_dir>/profiles/<name>/
    // But we return the conceptual path — the caller uses PraxisPaths
    // which resolves based on the active profile.
    PathBuf::from(format!("profiles/{name}"))
}

/// Ensure a profile's isolated directory structure exists.
pub fn ensure_profile_dirs(data_dir: &Path, name: &str) -> Result<PathBuf> {
    let profile_base = data_dir.join("profiles").join(name);
    let subdirs = &["tools", "skills", "memory", "sessions"];

    fs::create_dir_all(&profile_base)
        .with_context(|| format!("creating profile dir {}", profile_base.display()))?;

    for sub in subdirs {
        fs::create_dir_all(profile_base.join(sub)).with_context(|| format!("creating {}/", sub))?;
    }

    Ok(profile_base)
}

/// Get PraxisPaths scoped to a profile.
pub fn paths_for_profile(base_paths: &PraxisPaths, profile_name: &str) -> PraxisPaths {
    if profile_name == "default" {
        return base_paths.clone();
    }
    let profile_dir = base_paths.data_dir.join("profiles").join(profile_name);
    PraxisPaths::for_data_dir(profile_dir)
}

/// CLI handler for `praxis profile`.
pub fn handle_profile(data_dir: Option<PathBuf>, args: super::ProfileArgs) -> Result<String> {
    use crate::paths::default_data_dir;

    let data_dir_path = data_dir.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir_path);
    let mut profiles = Profiles::load(&paths.profiles_file)?;

    match args.command {
        super::ProfileCommand::List => {
            if profiles.profiles.is_empty() {
                return Ok("no profiles defined (using 'default').".to_string());
            }
            let mut lines = vec![format!("profiles (active: {}):", profiles.active)];
            for p in &profiles.profiles {
                let marker = if p.name == profiles.active { "→" } else { " " };
                lines.push(format!(
                    "  {} {} — {} {}",
                    marker,
                    p.name,
                    p.description,
                    if p.active { "[active]" } else { "" }
                ));
            }
            Ok(lines.join("\n"))
        }
        super::ProfileCommand::Create(create_args) => {
            profiles.create(&create_args.name, &create_args.description)?;
            profiles.save(&paths.profiles_file)?;
            ensure_profile_dirs(&paths.data_dir, &create_args.name)?;
            Ok(format!("created profile '{}'", create_args.name))
        }
        super::ProfileCommand::Switch(switch_args) => {
            let profile_path = profiles.switch(&switch_args.name)?;
            profiles.save(&paths.profiles_file)?;
            ensure_profile_dirs(&paths.data_dir, &switch_args.name)?;
            Ok(format!(
                "switched to profile '{}' ({})",
                switch_args.name,
                profile_path.display()
            ))
        }
        super::ProfileCommand::Remove(remove_args) => {
            let name = remove_args.name.clone();
            profiles.remove(&name)?;
            profiles.save(&paths.profiles_file)?;
            Ok(format!("removed profile '{}'", name))
        }
        super::ProfileCommand::Show(show_args) => {
            let p = profiles
                .get(&show_args.name)
                .ok_or_else(|| anyhow::anyhow!("profile '{}' not found", show_args.name))?;
            let mut lines = vec![
                format!("Profile: {}", p.name),
                format!("  description: {}", p.description),
                format!("  active: {}", p.active),
                format!("  created: {}", p.created_at),
            ];
            if !p.config_overrides.as_table().map(|t| t.is_empty()).unwrap_or(true) {
                lines.push(format!("  config overrides: {}", p.config_overrides));
            }
            Ok(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_switch_profile() {
        let mut profiles = Profiles::default();
        profiles.create("test-profile", "A test profile").unwrap();
        assert!(profiles.get("test-profile").is_some());

        profiles.switch("test-profile").unwrap();
        assert_eq!(profiles.active, "test-profile");
    }

    #[test]
    fn test_cannot_remove_default() {
        let mut profiles = Profiles::default();
        assert!(profiles.remove("default").is_err());
    }

    #[test]
    fn test_cannot_remove_active() {
        let mut profiles = Profiles::default();
        profiles.create("active-one", "active").unwrap();
        profiles.switch("active-one").unwrap();
        assert!(profiles.remove("active-one").is_err());
    }

    #[test]
    fn test_save_and_load() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("profiles.toml");

        let mut profiles = Profiles::default();
        profiles.create("save-test", "Test persistence").unwrap();
        profiles.save(&path).unwrap();

        let loaded = Profiles::load(&path).unwrap();
        assert!(loaded.get("save-test").is_some());
    }

    #[test]
    fn test_ensure_profile_dirs() {
        let tmp = tempdir().unwrap();
        let profile_base = ensure_profile_dirs(tmp.path(), "test-prof").unwrap();
        assert!(profile_base.join("tools").exists());
        assert!(profile_base.join("skills").exists());
        assert!(profile_base.join("memory").exists());
        assert!(profile_base.join("sessions").exists());
    }
}
