use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    paths::{PraxisPaths, default_data_dir},
    sandbox::{ChannelSandbox, ChannelSandboxStore},
};

#[derive(Debug, Args)]
pub struct SandboxArgs {
    #[command(subcommand)]
    command: SandboxCommands,
}

#[derive(Debug, Subcommand)]
enum SandboxCommands {
    /// List all configured channel sandbox policies.
    List,
    /// Show the policy for a specific channel.
    Show(ChannelArgs),
    /// Apply a preset policy to a channel.
    Set(SetSandboxArgs),
    /// Remove the sandbox policy for a channel.
    Remove(ChannelArgs),
}

#[derive(Debug, Args)]
struct ChannelArgs {
    /// Channel identifier (Telegram chat ID, delegation link name, etc.).
    channel: String,
}

#[derive(Debug, Args)]
struct SetSandboxArgs {
    /// Channel identifier.
    channel: String,
    /// Human-readable label for this sandbox.
    #[arg(long, default_value = "")]
    label: String,
    /// Preset to apply: strict, read-only, or custom.
    #[arg(long, default_value = "strict")]
    preset: String,
    /// Tool kinds to allow (repeatable; empty = use preset default).
    #[arg(long = "allow-kind")]
    allow_kinds: Vec<String>,
    /// Tool name glob patterns to deny (repeatable).
    #[arg(long = "deny-tool")]
    deny_tools: Vec<String>,
    /// Maximum security level (1–3).
    #[arg(long)]
    max_level: Option<u8>,
    /// Require approval for all tool calls.
    #[arg(long)]
    force_approval: bool,
}

pub(super) fn handle_sandbox(
    data_dir_override: Option<PathBuf>,
    args: SandboxArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        SandboxCommands::List => {
            let store = ChannelSandboxStore::load(&paths.sandbox_file)?;
            Ok(store.summary())
        }

        SandboxCommands::Show(a) => {
            let store = ChannelSandboxStore::load(&paths.sandbox_file)?;
            match store.get(&a.channel) {
                None => Ok(format!("sandbox: no policy for channel '{}'", a.channel)),
                Some(p) => {
                    let kinds = if p.allowed_tool_kinds.is_empty() {
                        "all".to_string()
                    } else {
                        p.allowed_tool_kinds.join(", ")
                    };
                    Ok(format!(
                        "channel: {}\nlabel: {}\nallowed_kinds: {}\ndenied_tools: {}\nmax_level: {}\nforce_approval: {}",
                        a.channel,
                        if p.label.is_empty() { "-" } else { &p.label },
                        kinds,
                        if p.denied_tool_name_patterns.is_empty() {
                            "none".to_string()
                        } else {
                            p.denied_tool_name_patterns.join(", ")
                        },
                        p.max_security_level
                            .map(|l| l.to_string())
                            .unwrap_or_else(|| "none".to_string()),
                        p.force_approval,
                    ))
                }
            }
        }

        SandboxCommands::Set(a) => {
            let mut sandbox = match a.preset.as_str() {
                "strict" => ChannelSandbox::strict(&a.label),
                "read-only" | "readonly" => ChannelSandbox::read_only(&a.label),
                "custom" => ChannelSandbox {
                    label: a.label.clone(),
                    ..Default::default()
                },
                other => {
                    anyhow::bail!("unknown preset '{other}'; use strict, read-only, or custom")
                }
            };

            // Apply overrides on top of the preset.
            if !a.allow_kinds.is_empty() {
                sandbox.allowed_tool_kinds = a.allow_kinds;
            }
            if !a.deny_tools.is_empty() {
                sandbox.denied_tool_name_patterns = a.deny_tools;
            }
            if let Some(level) = a.max_level {
                sandbox.max_security_level = Some(level);
            }
            if a.force_approval {
                sandbox.force_approval = true;
            }

            let mut store = ChannelSandboxStore::load(&paths.sandbox_file)?;
            store.set(&a.channel, sandbox);
            store.save(&paths.sandbox_file)?;
            Ok(format!(
                "sandbox: policy '{}' applied to channel '{}'",
                a.preset, a.channel
            ))
        }

        SandboxCommands::Remove(a) => {
            let mut store = ChannelSandboxStore::load(&paths.sandbox_file)?;
            if store.remove(&a.channel) {
                store.save(&paths.sandbox_file)?;
                Ok(format!(
                    "sandbox: policy removed for channel '{}'",
                    a.channel
                ))
            } else {
                Ok(format!(
                    "sandbox: no policy found for channel '{}'",
                    a.channel
                ))
            }
        }
    }
}
