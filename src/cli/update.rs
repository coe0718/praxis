//! Self-update CLI — check and install Praxis updates from GitHub releases.
//!
//! Supports `praxis update check` and `praxis update install` commands.

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

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

    let latest = release["tag_name"].as_str().unwrap_or("unknown").trim_start_matches('v');

    if latest == current {
        Ok(format!("You have the latest version {} (up to date)", current))
    } else {
        Ok(format!(
            "Update available: {} -> {}
Run `praxis update install` to upgrade",
            current, latest
        ))
    }
}

/// Find the current executable path.
fn current_exe() -> Result<PathBuf> {
    env::current_exe().context("could not determine current executable path")
}

/// Download the latest release binary for the current platform.
fn download_release() -> Result<(tempfile::NamedTempFile, String)> {
    let client = reqwest::blocking::Client::new();
    let url = "https://api.github.com/repos/coe0718/praxis/releases/latest";

    let response = client
        .get(url)
        .header("User-Agent", "praxis-update")
        .send()
        .context("failed to fetch release info")?;

    let release: serde_json::Value = response.json().context("failed to parse release info")?;

    let tag = release["tag_name"].as_str().context("missing tag_name in release")?;

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    };

    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let asset_name = format!("praxis-{}-{}.tar.gz", arch, os);

    let assets = release["assets"].as_array().context("no assets in release")?;

    let download_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(&asset_name))
        .and_then(|a| a["browser_download_url"].as_str())
        .unwrap_or_else(|| assets[0]["browser_download_url"].as_str().unwrap_or_default());

    if download_url.is_empty() {
        anyhow::bail!("could not find download URL for {} in release {}", asset_name, tag);
    }

    log::info!("downloading: {}", download_url);

    let resp = client
        .get(download_url)
        .header("User-Agent", "praxis-update")
        .send()
        .context("failed to download release")?;

    if !resp.status().is_success() {
        anyhow::bail!(
            "download failed with status {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        );
    }

    let bytes = resp.bytes().context("failed to read response bytes")?;

    let mut tmp = tempfile::NamedTempFile::new().context("failed to create temporary file")?;

    tmp.write_all(&bytes).context("failed to write download to temp file")?;

    Ok((tmp, tag.to_string()))
}

/// Extract the tarball and replace the current binary.
fn install_update(archive_path: &std::path::Path, new_version: &str) -> Result<()> {
    use std::process::Command;

    let current = current_exe()?;

    let extract_dir = tempfile::TempDir::new().context("failed to create extraction directory")?;

    let status = Command::new("tar")
        .args([
            "-xzf",
            archive_path.to_str().unwrap(),
            "-C",
            extract_dir.path().to_str().unwrap(),
        ])
        .status()
        .context("failed to extract update archive")?;

    if !status.success() {
        anyhow::bail!("tar extraction failed with code {:?}", status.code());
    }

    let binary_name = current
        .file_name()
        .and_then(|n| n.to_str())
        .context("could not determine binary name")?;

    let extracted = find_binary(extract_dir.path(), binary_name)?;

    fs::rename(&extracted, &current).context("failed to replace binary")?;

    let mut perms = fs::metadata(&current)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&current, perms)?;

    log::info!(
        "successfully updated to {} - binary replaced at {}",
        new_version,
        current.display()
    );

    Ok(())
}

/// Recursively search for a binary file by name.
fn find_binary(dir: &std::path::Path, name: &str) -> Result<PathBuf> {
    for entry in fs::read_dir(dir).context("failed to read extraction directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Ok(found) = find_binary(&path, name) {
                return Ok(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Ok(path);
        }
    }
    anyhow::bail!("could not find binary '{}' in extracted archive", name);
}

/// Current version string.
pub fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Install the latest update.
pub fn install_latest() -> Result<String> {
    let current = current_version();

    log::info!("update: checking for latest release...");
    let (archive, new_version) = download_release()?;

    if new_version.trim_start_matches('v') == current {
        return Ok(format!("Already on the latest version ({})", current));
    }

    log::info!("update: installing version {}...", new_version);
    install_update(archive.path(), &new_version)?;

    Ok(format!(
        "Successfully updated from {} to {}
Restart the daemon to use the new version.",
        current, new_version
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_is_set() {
        let v = current_version();
        assert!(!v.is_empty(), "version should not be empty");
    }
}
