use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::time::parse_timezone;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub instance: InstanceConfig,
    pub runtime: RuntimeConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub agent: AgentConfig,
    #[serde(default)]
    pub context: ContextConfig,
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
    pub model_pin: Option<String>,
    #[serde(default)]
    pub local_first_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextConfig {
    pub window_tokens: usize,
    pub budget: Vec<ContextSourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextSourceConfig {
    pub source: String,
    pub priority: u8,
    pub max_pct: f32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            window_tokens: 12_000,
            budget: vec![
                source("identity", 1, 0.05),
                source("agents", 2, 0.05),
                source("active_goals", 3, 0.10),
                source("do_not_repeat", 4, 0.08),
                source("known_bugs", 5, 0.07),
                source("memory_hot", 6, 0.18),
                source("memory_cold", 7, 0.12),
                source("predictions", 8, 0.05),
                source("patterns", 9, 0.05),
                source("journal", 10, 0.05),
                source("tools", 11, 0.05),
                source("task", 12, 0.20),
            ],
        }
    }
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
                model_pin: None,
                local_first_fallback: false,
            },
            context: ContextConfig::default(),
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

fn source(name: &str, priority: u8, max_pct: f32) -> ContextSourceConfig {
    ContextSourceConfig {
        source: name.to_string(),
        priority,
        max_pct,
    }
}
