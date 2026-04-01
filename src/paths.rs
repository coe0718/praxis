use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOs,
    Other,
}

#[derive(Debug, Clone)]
pub struct PraxisPaths {
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub database_file: PathBuf,
    pub state_file: PathBuf,
    pub identity_file: PathBuf,
    pub goals_file: PathBuf,
    pub roadmap_file: PathBuf,
    pub journal_file: PathBuf,
    pub metrics_file: PathBuf,
    pub patterns_file: PathBuf,
    pub learnings_file: PathBuf,
    pub capabilities_file: PathBuf,
    pub proposals_file: PathBuf,
    pub day_count_file: PathBuf,
    pub goals_dir: PathBuf,
    pub goals_criteria_dir: PathBuf,
    pub tools_dir: PathBuf,
}

pub fn detect_platform() -> Platform {
    if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Other
    }
}

pub fn default_data_dir() -> Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set, unable to resolve a default Praxis data directory")?;
    let xdg_data_home = env::var_os("XDG_DATA_HOME").map(PathBuf::from);

    Ok(default_data_dir_for(
        detect_platform(),
        &home,
        xdg_data_home.as_deref(),
    ))
}

pub fn default_data_dir_for(
    platform: Platform,
    home: &Path,
    xdg_data_home: Option<&Path>,
) -> PathBuf {
    match platform {
        Platform::Linux => xdg_data_home
            .map(|dir| dir.join("praxis"))
            .unwrap_or_else(|| home.join(".local/share/praxis")),
        Platform::MacOs => home.join("Library/Application Support/Praxis"),
        Platform::Other => home.join(".praxis"),
    }
}

impl PraxisPaths {
    pub fn for_data_dir(data_dir: PathBuf) -> Self {
        Self::new(
            data_dir,
            PathBuf::from("praxis.db"),
            PathBuf::from("session_state.json"),
        )
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self::new(
            config.instance.data_dir.clone(),
            config.database.path.clone(),
            config.runtime.state_file.clone(),
        )
    }

    pub fn new(data_dir: PathBuf, database_path: PathBuf, state_file: PathBuf) -> Self {
        let goals_dir = data_dir.join("goals");
        let goals_criteria_dir = goals_dir.join("criteria");
        let tools_dir = data_dir.join("tools");

        Self {
            config_file: data_dir.join("praxis.toml"),
            database_file: resolve_path(&data_dir, &database_path),
            state_file: resolve_path(&data_dir, &state_file),
            identity_file: data_dir.join("IDENTITY.md"),
            goals_file: data_dir.join("GOALS.md"),
            roadmap_file: data_dir.join("ROADMAP.md"),
            journal_file: data_dir.join("JOURNAL.md"),
            metrics_file: data_dir.join("METRICS.md"),
            patterns_file: data_dir.join("PATTERNS.md"),
            learnings_file: data_dir.join("LEARNINGS.md"),
            capabilities_file: data_dir.join("CAPABILITIES.md"),
            proposals_file: data_dir.join("PROPOSALS.md"),
            day_count_file: data_dir.join("DAY_COUNT"),
            goals_dir,
            goals_criteria_dir,
            tools_dir,
            data_dir,
        }
    }

    pub fn identity_files(&self) -> Vec<PathBuf> {
        vec![
            self.identity_file.clone(),
            self.goals_file.clone(),
            self.roadmap_file.clone(),
            self.journal_file.clone(),
            self.metrics_file.clone(),
            self.patterns_file.clone(),
            self.learnings_file.clone(),
            self.capabilities_file.clone(),
            self.proposals_file.clone(),
            self.day_count_file.clone(),
        ]
    }
}

fn resolve_path(base: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Platform, default_data_dir_for};

    #[test]
    fn linux_prefers_xdg_data_home() {
        let home = PathBuf::from("/home/tester");
        let xdg = PathBuf::from("/tmp/data-home");

        let path = default_data_dir_for(Platform::Linux, &home, Some(&xdg));
        assert_eq!(path, PathBuf::from("/tmp/data-home/praxis"));
    }

    #[test]
    fn macos_uses_application_support() {
        let home = PathBuf::from("/Users/tester");
        let path = default_data_dir_for(Platform::MacOs, &home, None);

        assert_eq!(
            path,
            PathBuf::from("/Users/tester/Library/Application Support/Praxis")
        );
    }
}
