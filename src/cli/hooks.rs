use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    hooks::{HookEntry, HookKind, HookRunner, install_hook, remove_hook},
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct HooksArgs {
    #[command(subcommand)]
    command: HooksCommands,
}

#[derive(Debug, Subcommand)]
enum HooksCommands {
    /// List all registered hooks.
    List,
    /// Register a new hook.
    Add(AddHookArgs),
    /// Remove a hook by its script path.
    Remove(RemoveHookArgs),
    /// Fire a named event to test observer hooks (dry-run for interceptors).
    Test(TestHookArgs),
}

#[derive(Debug, Args)]
struct AddHookArgs {
    /// Event name to listen for (e.g. `session.end`, `phase.*`).
    #[arg(long)]
    event: String,
    /// Hook kind: observer, interceptor, or approval.
    #[arg(long, default_value = "observer")]
    kind: String,
    /// Absolute path to the script to run.
    #[arg(long)]
    script: PathBuf,
    /// Optional glob filter on tool name or phase (default: match all).
    #[arg(long)]
    filter: Option<String>,
    /// Timeout in seconds (default: 10).
    #[arg(long, default_value_t = 10)]
    timeout: u64,
}

#[derive(Debug, Args)]
struct RemoveHookArgs {
    /// Absolute path of the script to remove.
    script: PathBuf,
}

#[derive(Debug, Args)]
struct TestHookArgs {
    /// Event name to fire.
    event: String,
}

pub(super) fn handle_hooks(data_dir_override: Option<PathBuf>, args: HooksArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        HooksCommands::List => {
            let runner = HookRunner::load(&paths.hooks_file)?;
            if runner.is_empty() {
                return Ok("hooks: none registered".to_string());
            }
            // Re-load as raw config for display.
            let config: crate::hooks::HookConfig = if paths.hooks_file.exists() {
                let raw = std::fs::read_to_string(&paths.hooks_file)?;
                toml::from_str(&raw)?
            } else {
                Default::default()
            };
            let lines: Vec<String> = config
                .hooks
                .iter()
                .map(|h| {
                    format!(
                        "  [{:?}] {} → {} (filter: {}, timeout: {}s)",
                        h.kind,
                        h.event,
                        h.script.display(),
                        h.filter.as_deref().unwrap_or("*"),
                        h.timeout_secs,
                    )
                })
                .collect();
            Ok(format!(
                "hooks: {} registered\n{}",
                lines.len(),
                lines.join("\n")
            ))
        }

        HooksCommands::Add(a) => {
            let kind = parse_kind(&a.kind)?;
            let entry = HookEntry {
                event: a.event.clone(),
                kind,
                script: a.script.clone(),
                filter: a.filter,
                timeout_secs: a.timeout,
            };
            install_hook(&paths, entry)?;
            Ok(format!(
                "hooks: registered {:?} hook for '{}' → {}",
                kind,
                a.event,
                a.script.display()
            ))
        }

        HooksCommands::Remove(a) => {
            let removed = remove_hook(&paths, &a.script)?;
            if removed > 0 {
                Ok(format!(
                    "hooks: removed {removed} hook(s) matching {}",
                    a.script.display()
                ))
            } else {
                Ok(format!("hooks: no hooks found for {}", a.script.display()))
            }
        }

        HooksCommands::Test(a) => {
            let runner = HookRunner::load(&paths.hooks_file)?;
            let ctx = crate::hooks::HookContext::new(&a.event, paths.data_dir.clone());
            runner.fire_observer(&a.event, &ctx, "*");
            Ok(format!(
                "hooks: fired '{}' (observers only — interceptors not run in test)",
                a.event
            ))
        }
    }
}

fn parse_kind(s: &str) -> Result<HookKind> {
    match s {
        "observer" => Ok(HookKind::Observer),
        "interceptor" => Ok(HookKind::Interceptor),
        "approval" => Ok(HookKind::Approval),
        other => {
            anyhow::bail!("unknown hook kind '{other}'; use observer, interceptor, or approval")
        }
    }
}
