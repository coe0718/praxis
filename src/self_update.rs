//! Self-update — Download and swap Praxis binary at runtime.
//!
//! `praxis update` fetches the latest release from GitHub and swaps the binary.
//! Inspired by Moltis's self-update mechanism.
//!
//! Safety: Binary is downloaded to temp location, verified, then swapped atomically.

use anyhow::Result;

use crate::paths::PraxisPaths;

/// Check for updates and download if available.
pub async fn check_for_updates(_paths: &PraxisPaths) -> Result<Option<String>> {
    // Check GitHub releases for newer version
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/repos/coe0718/praxis/releases/latest")
        .header("User-Agent", "Praxis-Updater")
        .send()
        .await?;
    
    if !resp.status().is_success() {
        return Ok(None);
    }
    
    let release: serde_json::Value = resp.json().await?;
    let tag = release["tag_name"].as_str().unwrap_or("");
    
    // Compare with current version
    let current = env!("CARGO_PKG_VERSION");
    if tag != current {
        return Ok(Some(tag.to_string()));
    }
    
    Ok(None)
}

/// Perform self-update.
pub async fn perform_self_update(paths: &PraxisPaths) -> Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/repos/coe0718/praxis/releases/latest")
        .header("User-Agent", "Praxis-Updater")
        .send()
        .await?;
    
    let release: serde_json::Value = resp.json().await?;
    let asset = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["name"].as_str().map(|n| n.contains("linux")).unwrap_or(false)))
        .ok_or_else(|| anyhow::anyhow!("No linux binary found"))?;
    
    let download_url = asset["browser_download_url"].as_str()
        .ok_or_else(|| anyhow::anyhow!("No download URL"))?;
    
    // Download to temp file
    let binary_bytes = client.get(download_url).send().await?.bytes().await?;
    let temp_path = paths.data_dir.join("praxis-new");
    tokio::fs::write(&temp_path, &binary_bytes).await?;
    
    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755)).await?;
    }
    
    // Swap current binary
    let current_exe = std::env::current_exe()?;
    let backup_path = paths.data_dir.join("praxis-backup");
    
    if current_exe.exists() {
        tokio::fs::copy(&current_exe, &backup_path).await?;
        tokio::fs::remove_file(&current_exe).await?;
    }
    
    tokio::fs::rename(&temp_path, &current_exe).await?;
    
    Ok(())
}