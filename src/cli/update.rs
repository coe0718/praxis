//! Self-update CLI — check and install Praxis updates from GitHub releases.

use anyhow::{Context, Result};
use std::env;

/// Arguments for the update command.
#[derive(Debug, Clone, clap::Args)]
pub struct UpdateArgs {
    #[command(subcommand)]
    pub command: UpdateCommand,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum UpdateCommand {
    /// Check for available updates from GitHub.
    Check,
    /// Install the latest update (downloads binary and replaces current).
    Install,
}

/// Check for updates by comparing the current version with the latest GitHub release.
pub fn check_for_update() -> Result<String> {
    let current = env!("CARGO_PKG_VERSION");

    let client = reqwest::blocking::Client::new();
    let url = "https://api.github.com/repos/coe0718/praxis/releases/latest";

    let response = client
        .get(url)
        .header("User-Agent", "praxis-update")
        .send()
        .context("failed to check for updates")?;

    if !response.status().is_success() {
        return Ok(format!("You have version {} (could not check for updates)", current));
    }

    let release: serde_json::Value = response.json().context("failed to parse release info")?;

    let latest = release["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v');

    if latest == current {
        Ok(format!("You have the latest version {} (up to date)", current))
    } else {
        Ok(format!(
            "Update available: {} → {}\nRun `praxis update install` to upgrade",
            current, latest
        ))
    }
}

/// Current version string.
#[allow(dead_code)]
pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}