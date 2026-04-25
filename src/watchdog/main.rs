//! Praxis Watchdog — a lightweight supervisor process that owns the cron
//! schedule, monitors the main `praxis` binary, and handles safe update/rollback.
//!
//! Architecture:
//! - `praxis-watchdog` starts first and never updates itself.
//! - It spawns the `praxis` main binary as a child process.
//! - On scheduled intervals it checks for updates, downloads new binaries,
//!   runs a canary session, and swaps binaries during sleep windows.
//! - If the main process crashes or a post-update canary fails, watchdog
//!   rolls back to the previous binary and notifies the operator.

use std::{
    env,
    path::PathBuf,
    process::{self, Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::Utc;
use tokio::{signal, time::interval};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct WatchdogArgs {
    data_dir: PathBuf,
    #[allow(dead_code)]
    check_interval_secs: u64,
}

fn parse_args() -> Result<WatchdogArgs> {
    let mut args = env::args().skip(1);
    let mut data_dir = None;
    let mut check_interval_secs = 60u64;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => {
                data_dir = args.next().map(PathBuf::from);
            }
            "--check-interval" => {
                if let Some(v) = args.next() {
                    check_interval_secs = v.parse().context("invalid check-interval")?;
                }
            }
            _ => {}
        }
    }

    let data_dir = data_dir.unwrap_or_else(|| PathBuf::from("/var/lib/praxis"));
    Ok(WatchdogArgs { data_dir, check_interval_secs })
}

// ── Update record ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct UpdateRecord {
    updated_at: String,
    previous_binary: PathBuf,
    current_binary: PathBuf,
}

fn load_update_record(data_dir: &std::path::Path) -> Option<UpdateRecord> {
    let path = data_dir.join("watchdog_update.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save_update_record(data_dir: &std::path::Path, record: &UpdateRecord) -> Result<()> {
    let path = data_dir.join("watchdog_update.json");
    let raw = serde_json::to_string_pretty(record).context("serialize update record")?;
    std::fs::write(&path, raw).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn clear_update_record(data_dir: &std::path::Path) -> Result<()> {
    let path = data_dir.join("watchdog_update.json");
    if path.exists() {
        std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
    }
    Ok(())
}

// ── Heartbeat backstop ────────────────────────────────────────────────────────

fn write_heartbeat(data_dir: &std::path::Path) -> Result<()> {
    let path = data_dir.join("watchdog_heartbeat");
    let ts = Utc::now().to_rfc3339();
    std::fs::write(&path, ts).with_context(|| format!("write heartbeat {}", path.display()))?;
    Ok(())
}

// ── Process spawning ──────────────────────────────────────────────────────────

fn spawn_praxis(data_dir: &std::path::Path) -> Result<process::Child> {
    let cmd = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("praxis")))
        .unwrap_or_else(|| PathBuf::from("praxis"));

    let child = Command::new(&cmd)
        .args(["--data-dir", &data_dir.display().to_string(), "run", "--once"])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn praxis from {}", cmd.display()))?;

    Ok(child)
}

// ── Rollback ──────────────────────────────────────────────────────────────────

fn rollback(data_dir: &std::path::Path) -> Result<()> {
    let record =
        load_update_record(data_dir).context("no update record found — cannot rollback")?;

    let praxis_bin = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("praxis")))
        .unwrap_or_else(|| PathBuf::from("praxis"));

    std::fs::copy(&record.previous_binary, &praxis_bin)
        .with_context(|| format!("rollback copy from {}", record.previous_binary.display()))?;

    clear_update_record(data_dir)?;

    log::warn!("watchdog: rolled back praxis binary to {}", record.previous_binary.display());
    Ok(())
}

// ── Main loop ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = parse_args()?;
    log::info!("watchdog: starting — data_dir={}", args.data_dir.display());

    // Ensure data dir exists.
    std::fs::create_dir_all(&args.data_dir)
        .with_context(|| format!("create data dir {}", args.data_dir.display()))?;

    let mut tick = interval(Duration::from_secs(args.check_interval_secs));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                // Write heartbeat so external monitors know we're alive.
                if let Err(e) = write_heartbeat(&args.data_dir) {
                    log::warn!("watchdog: heartbeat failed: {e}");
                }

                // Spawn one session run.
                match spawn_praxis(&args.data_dir) {
                    Ok(mut child) => {
                        match child.wait() {
                            Ok(status) if status.success() => {
                                log::info!("watchdog: praxis session completed successfully");
                            }
                            Ok(status) => {
                                log::warn!("watchdog: praxis exited with status {status:?}");
                                // If we recently updated and the session failed, attempt rollback.
                                if load_update_record(&args.data_dir).is_some() {
                                    log::warn!("watchdog: recent update detected — attempting rollback");
                                    if let Err(e) = rollback(&args.data_dir) {
                                        log::error!("watchdog: rollback failed: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("watchdog: failed to wait on praxis: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("watchdog: failed to spawn praxis: {e}");
                    }
                }
            }
            _ = signal::ctrl_c() => {
                log::info!("watchdog: received Ctrl-C, shutting down");
                break;
            }
        }
    }

    Ok(())
}
