use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::paths::{PraxisPaths, default_data_dir};

#[derive(Debug, Args)]
pub struct WatchdogArgs {
    #[command(subcommand)]
    command: WatchdogCommand,
}

#[derive(Debug, Subcommand)]
enum WatchdogCommand {
    /// Install the Praxis agent as a system service.
    Install(WatchdogInstallArgs),
    /// Remove the installed system service.
    Uninstall,
    /// Show current watchdog service status.
    Status,
}

#[derive(Debug, Args)]
struct WatchdogInstallArgs {
    /// How often to run the agent loop (in seconds, minimum 60).
    #[arg(long, default_value_t = 300)]
    interval_secs: u64,

    /// User to run the service as (systemd only; defaults to current user).
    #[arg(long)]
    user: Option<String>,
}

pub(crate) fn handle_watchdog(
    data_dir_override: Option<PathBuf>,
    args: WatchdogArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        WatchdogCommand::Install(a) => watchdog_install(&paths, a),
        WatchdogCommand::Uninstall => watchdog_uninstall(&paths),
        WatchdogCommand::Status => watchdog_status(&paths),
    }
}

// ---------------------------------------------------------------------------
// Platform dispatch
// ---------------------------------------------------------------------------

fn watchdog_install(paths: &PraxisPaths, args: WatchdogInstallArgs) -> Result<String> {
    let interval = args.interval_secs.max(60);

    #[cfg(target_os = "macos")]
    {
        install_launchd(paths, interval)
    }
    #[cfg(target_os = "linux")]
    {
        install_systemd(paths, interval, args.user.as_deref())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (paths, interval, args);
        bail!("watchdog is only supported on Linux and macOS");
    }
}

fn watchdog_uninstall(paths: &PraxisPaths) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        uninstall_launchd(paths)
    }
    #[cfg(target_os = "linux")]
    {
        uninstall_systemd(paths)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = paths;
        bail!("watchdog is only supported on Linux and macOS");
    }
}

fn watchdog_status(paths: &PraxisPaths) -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        status_launchd(paths)
    }
    #[cfg(target_os = "linux")]
    {
        status_systemd(paths)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = paths;
        bail!("watchdog is only supported on Linux and macOS");
    }
}

// ---------------------------------------------------------------------------
// macOS — launchd
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn plist_label() -> &'static str {
    "com.praxis.agent"
}

#[cfg(target_os = "macos")]
fn plist_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home)
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", plist_label()))
}

#[cfg(target_os = "macos")]
fn praxis_exe() -> Result<String> {
    which_praxis()
}

#[cfg(target_os = "macos")]
fn install_launchd(paths: &PraxisPaths, interval_secs: u64) -> Result<String> {
    let exe = praxis_exe()?;
    let data_dir = paths.data_dir.display().to_string();
    let label = plist_label();
    let path = plist_path();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe}</string>
        <string>--data-dir</string>
        <string>{data_dir}</string>
        <string>run</string>
        <string>--once</string>
    </array>
    <key>StartInterval</key>
    <integer>{interval_secs}</integer>
    <key>RunAtLoad</key>
    <false/>
    <key>StandardOutPath</key>
    <string>{data_dir}/watchdog.log</string>
    <key>StandardErrorPath</key>
    <string>{data_dir}/watchdog.log</string>
</dict>
</plist>
"#
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, &plist).with_context(|| format!("failed to write {}", path.display()))?;

    // Unload first in case it was previously installed.
    let _ = Command::new("launchctl")
        .args(["unload", &path.to_string_lossy()])
        .output();

    let load = Command::new("launchctl")
        .args(["load", &path.to_string_lossy()])
        .output()
        .context("failed to run launchctl load")?;

    if !load.status.success() {
        let err = String::from_utf8_lossy(&load.stderr);
        bail!("launchctl load failed: {err}");
    }

    Ok(format!(
        "watchdog: installed (launchd)\nlabel: {label}\nplist: {}\ninterval: {interval_secs}s",
        path.display()
    ))
}

#[cfg(target_os = "macos")]
fn uninstall_launchd(_paths: &PraxisPaths) -> Result<String> {
    let path = plist_path();
    let label = plist_label();

    if !path.exists() {
        return Ok(format!(
            "watchdog: not installed (no plist at {})",
            path.display()
        ));
    }

    let _ = Command::new("launchctl")
        .args(["unload", &path.to_string_lossy()])
        .output();

    fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;

    Ok(format!("watchdog: uninstalled\nlabel: {label}"))
}

#[cfg(target_os = "macos")]
fn status_launchd(_paths: &PraxisPaths) -> Result<String> {
    let label = plist_label();
    let out = Command::new("launchctl")
        .args(["list", label])
        .output()
        .context("failed to run launchctl list")?;

    if !out.status.success() {
        return Ok(format!("watchdog: not loaded (label {label})"));
    }

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(format!("watchdog: loaded\n{stdout}"))
}

// ---------------------------------------------------------------------------
// Linux — systemd user unit
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn unit_path(user: Option<&str>) -> Result<PathBuf> {
    if user.is_some() {
        // System-level unit.
        Ok(PathBuf::from("/etc/systemd/system/praxis-agent.service"))
    } else {
        // User-level unit.
        let home = std::env::var("HOME").context("HOME is not set")?;
        Ok(PathBuf::from(home)
            .join(".config/systemd/user")
            .join("praxis-agent.service"))
    }
}

#[cfg(target_os = "linux")]
fn install_systemd(paths: &PraxisPaths, interval_secs: u64, user: Option<&str>) -> Result<String> {
    let exe = which_praxis()?;
    let data_dir = paths.data_dir.display().to_string();
    let path = unit_path(user)?;
    let run_user = user
        .map(str::to_string)
        .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "nobody".to_string()));

    let unit = format!(
        "[Unit]\n\
         Description=Praxis AI Agent\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=oneshot\n\
         User={run_user}\n\
         ExecStart={exe} --data-dir {data_dir} run --once\n\
         StandardOutput=append:{data_dir}/watchdog.log\n\
         StandardError=append:{data_dir}/watchdog.log\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n"
    );

    let timer = format!(
        "[Unit]\n\
         Description=Praxis AI Agent timer\n\
         \n\
         [Timer]\n\
         OnBootSec=60\n\
         OnUnitActiveSec={interval_secs}\n\
         Unit=praxis-agent.service\n\
         \n\
         [Install]\n\
         WantedBy=timers.target\n"
    );

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let timer_path = path.with_extension("timer");
    fs::write(&path, &unit).with_context(|| format!("failed to write {}", path.display()))?;
    fs::write(&timer_path, &timer)
        .with_context(|| format!("failed to write {}", timer_path.display()))?;

    let systemctl = if user.is_some() {
        vec!["systemctl"]
    } else {
        vec!["systemctl", "--user"]
    };

    run_systemctl(&systemctl, &["daemon-reload"])?;
    run_systemctl(&systemctl, &["enable", "--now", "praxis-agent.timer"])?;

    Ok(format!(
        "watchdog: installed (systemd)\nunit: {}\ntimer: {}\ninterval: {interval_secs}s",
        path.display(),
        timer_path.display()
    ))
}

#[cfg(target_os = "linux")]
fn uninstall_systemd(_paths: &PraxisPaths) -> Result<String> {
    let user_path = unit_path(None)?;
    let system_path = PathBuf::from("/etc/systemd/system/praxis-agent.service");

    for (path, args) in [
        (&user_path, vec!["systemctl", "--user"]),
        (&system_path, vec!["systemctl"]),
    ] {
        if path.exists() {
            let timer = path.with_extension("timer");
            let _ = run_systemctl(&args, &["disable", "--now", "praxis-agent.timer"]);
            let _ = fs::remove_file(&timer);
            fs::remove_file(path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            let _ = run_systemctl(&args, &["daemon-reload"]);
            return Ok(format!("watchdog: uninstalled\nunit: {}", path.display()));
        }
    }

    Ok("watchdog: not installed".to_string())
}

#[cfg(target_os = "linux")]
fn status_systemd(_paths: &PraxisPaths) -> Result<String> {
    let out = Command::new("systemctl")
        .args(["--user", "status", "praxis-agent.timer"])
        .output()
        .context("failed to run systemctl status")?;

    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(format!("watchdog: active\n{stdout}"))
    } else {
        // Try system-level.
        let out2 = Command::new("systemctl")
            .args(["status", "praxis-agent.timer"])
            .output()
            .context("failed to run systemctl status")?;

        if out2.status.success() {
            Ok(format!(
                "watchdog: active (system)\n{}",
                String::from_utf8_lossy(&out2.stdout).trim()
            ))
        } else {
            Ok("watchdog: not active".to_string())
        }
    }
}

#[cfg(target_os = "linux")]
fn run_systemctl(base: &[&str], args: &[&str]) -> Result<()> {
    let mut all_args: Vec<&str> = base[1..].to_vec();
    all_args.extend_from_slice(args);
    let out = Command::new(base[0])
        .args(&all_args)
        .output()
        .with_context(|| format!("failed to run {} {}", base[0], args.join(" ")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("{} {} failed: {err}", base[0], args.join(" "));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helper
// ---------------------------------------------------------------------------

fn which_praxis() -> Result<String> {
    // Prefer the current executable, fall back to PATH lookup.
    if let Ok(exe) = std::env::current_exe() {
        return Ok(exe.to_string_lossy().to_string());
    }
    let out = Command::new("which")
        .arg("praxis")
        .output()
        .context("failed to locate praxis executable")?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        bail!("could not locate the praxis executable — make sure it is in PATH");
    }
}
