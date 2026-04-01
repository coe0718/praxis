use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::time::{parse_clock_time, parse_timezone};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub instance: InstanceConfig,
    pub runtime: RuntimeConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstanceConfig {
    pub name: String,
    pub timezone: String,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    pub quiet_hours_start: String,
    pub quiet_hours_end: String,
    pub state_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityConfig {
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    pub backend: String,
    pub context_ceiling_pct: f32,
}

impl AppConfig {
    pub fn default_for_data_dir(data_dir: PathBuf) -> Self {
        Self {
            instance: InstanceConfig {
                name: "Praxis".to_string(),
                timezone: "UTC".to_string(),
                data_dir,
            },
            runtime: RuntimeConfig {
                quiet_hours_start: "23:00".to_string(),
                quiet_hours_end: "07:00".to_string(),
                state_file: PathBuf::from("session_state.json"),
            },
            database: DatabaseConfig {
                path: PathBuf::from("praxis.db"),
            },
            security: SecurityConfig { level: 2 },
            agent: AgentConfig {
                backend: "stub".to_string(),
                context_ceiling_pct: 0.80,
            },
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config from {}", path.display()))?;
        let config: Self =
            toml::from_str(&raw).with_context(|| format!("invalid TOML in {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("failed to serialize praxis config")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.instance.name.trim().is_empty() {
            bail!("instance.name must not be empty");
        }

        parse_timezone(&self.instance.timezone)?;
        parse_clock_time(&self.runtime.quiet_hours_start)?;
        parse_clock_time(&self.runtime.quiet_hours_end)?;

        if !(1..=4).contains(&self.security.level) {
            bail!("security.level must be between 1 and 4");
        }

        if self.agent.backend != "stub" {
            bail!(
                "agent.backend must be \"stub\" for the foundation milestone, got {}",
                self.agent.backend
            );
        }

        if !(0.0..=1.0).contains(&self.agent.context_ceiling_pct)
            || self.agent.context_ceiling_pct == 0.0
        {
            bail!("agent.context_ceiling_pct must be greater than 0.0 and at most 1.0");
        }

        if self.database.path.as_os_str().is_empty() {
            bail!("database.path must not be empty");
        }

        if self.runtime.state_file.as_os_str().is_empty() {
            bail!("runtime.state_file must not be empty");
        }

        Ok(())
    }

    pub fn with_overridden_data_dir(mut self, data_dir: PathBuf) -> Self {
        self.instance.data_dir = data_dir;
        self
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.instance.name = name.into();
    }

    pub fn set_timezone(&mut self, timezone: impl Into<String>) -> Result<()> {
        let timezone = timezone.into();
        parse_timezone(&timezone)?;
        self.instance.timezone = timezone;
        Ok(())
    }
}

impl FromStr for AppConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let config: Self = toml::from_str(s).context("invalid Praxis config")?;
        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::AppConfig;

    #[test]
    fn saves_and_loads_valid_config() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("praxis.toml");
        let config = AppConfig::default_for_data_dir(temp.path().join("data"));

        config.save(&path).unwrap();
        let loaded = AppConfig::load(&path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn rejects_invalid_backend() {
        let mut config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        config.agent.backend = "claude".to_string();

        let error = config.validate().unwrap_err().to_string();
        assert!(error.contains("agent.backend"));
    }
}
