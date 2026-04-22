use std::{fs, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Sensitive configuration overrides stored separately from `praxis.toml`.
///
/// `security.toml` should be added to `.gitignore` so that uncommittable
/// material such as security-level overrides never lands in version control.
/// All fields are optional — absent fields leave the base config unchanged.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityOverrides {
    /// Override the security level from `praxis.toml`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// Optional operator note — free-text memo for why overrides are in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl SecurityOverrides {
    /// Load overrides from `path`, returning an empty struct if the file is absent.
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        let overrides = toml::from_str::<Self>(&raw)?;
        // Validate that level, if present, is in the valid range 1–3.
        if let Some(level) = overrides.level && (level == 0 || level > 3) {
            anyhow::bail!(
                "security override level must be between 1 and 3 (inclusive), got {level}"
            );
        }
        Ok(overrides)
    }

    pub fn is_empty(&self) -> bool {
        self.level.is_none() && self.note.is_none()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::SecurityOverrides;

    #[test]
    fn returns_default_when_file_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("security.toml");
        let overrides = SecurityOverrides::load_or_default(&path).unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn loads_level_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("security.toml");
        std::fs::write(&path, "level = 3\n").unwrap();
        let overrides = SecurityOverrides::load_or_default(&path).unwrap();
        assert_eq!(overrides.level, Some(3));
    }

    #[test]
    fn rejects_out_of_bounds_level() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("security.toml");
        std::fs::write(&path, "level = 0\n").unwrap();
        assert!(SecurityOverrides::load_or_default(&path).is_err());
        std::fs::write(&path, "level = 4\n").unwrap();
        assert!(SecurityOverrides::load_or_default(&path).is_err());
    }
}
