use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    daemon::{DaemonConfig, daemon_is_running, daemon_status_line, read_daemon_pid, run_daemon},
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    command: Option<DaemonCommands>,

    /// How often to poll for new bus events and wake intents (seconds).
    #[arg(long, default_value_t = 30)]
    poll_interval: u64,

    /// Minimum gap between scheduled sessions (minutes).
    #[arg(long, default_value_t = 240)]
    session_gap: u64,

    /// How long between maintenance windows (minutes).
    #[arg(long, default_value_t = 60)]
    maintenance_interval: u64,

    /// Override an existing PID file and start even if another daemon may be
    /// running.  Use only when you are sure the old process is gone.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum DaemonCommands {
    /// Show whether the daemon is currently running.
    Status,
    /// Send a graceful stop signal to the running daemon.
    Stop,
}

pub(super) fn handle_daemon(
    data_dir_override: Option<PathBuf>,
    args: DaemonArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir.clone());

    match args.command {
        Some(DaemonCommands::Status) => Ok(daemon_status_line(&paths)),

        Some(DaemonCommands::Stop) => {
            let pid = match read_daemon_pid(&paths) {
                Some(p) => p,
                None => return Ok("daemon: no PID file found — daemon is not running".to_string()),
            };
            if !daemon_is_running(&paths) {
                return Ok(format!(
                    "daemon: stale PID file (pid={pid}) — daemon is not running"
                ));
            }
            send_sigterm(pid)?;
            Ok(format!("daemon: SIGTERM sent to pid={pid}"))
        }

        None => {
            // No subcommand → start the daemon.
            let config = DaemonConfig {
                poll_interval_secs: args.poll_interval,
                min_session_gap_mins: args.session_gap,
                maintenance_interval_mins: args.maintenance_interval,
                force_start: args.force,
            };
            // run_daemon blocks until shutdown.
            run_daemon(data_dir, config)?;
            Ok("daemon: exited".to_string())
        }
    }
}

fn send_sigterm(pid: u32) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        // `/proc/<pid>/status` existing means we can signal it.
        // Use `kill -TERM <pid>` via std::process::Command — no libc needed.
        let status = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .context("failed to run kill command")?;
        if !status.success() {
            anyhow::bail!("kill returned non-zero for pid={pid}");
        }
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        anyhow::bail!("daemon stop is not supported on this platform")
    }
}

use anyhow::Context;
