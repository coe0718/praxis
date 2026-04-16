use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    delegation::{DelegationLink, DelegationStore, LinkDirection},
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct DelegationArgs {
    #[command(subcommand)]
    command: DelegationCommands,
}

#[derive(Debug, Subcommand)]
enum DelegationCommands {
    /// List all configured delegation links.
    List,
    /// Add a new delegation link.
    Add(AddLinkArgs),
    /// Remove a delegation link by name.
    Remove(RemoveLinkArgs),
    /// Enable or disable a delegation link.
    Enable(ToggleLinkArgs),
    Disable(ToggleLinkArgs),
}

#[derive(Debug, Args)]
struct AddLinkArgs {
    /// Human-readable name for this link.
    #[arg(long)]
    name: String,
    /// Endpoint identifier — URL, agent name, or channel address.
    #[arg(long)]
    endpoint: String,
    /// Link direction: outbound, inbound, or bidirectional.
    #[arg(long, default_value = "outbound")]
    direction: String,
    /// Maximum concurrent tasks on this link.
    #[arg(long, default_value_t = 1)]
    concurrency: usize,
    /// Glob patterns of task names to allow (repeatable; empty = allow all).
    #[arg(long = "allow")]
    allow: Vec<String>,
    /// Glob patterns of task names to deny (repeatable).
    #[arg(long = "deny")]
    deny: Vec<String>,
}

#[derive(Debug, Args)]
struct RemoveLinkArgs {
    /// Name of the link to remove.
    name: String,
}

#[derive(Debug, Args)]
struct ToggleLinkArgs {
    /// Name of the link to toggle.
    name: String,
}

pub(super) fn handle_delegation(
    data_dir_override: Option<PathBuf>,
    args: DelegationArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        DelegationCommands::List => {
            let store = DelegationStore::load(&paths.delegation_links_file)?;
            Ok(store.summary())
        }
        DelegationCommands::Add(a) => {
            let direction = parse_direction(&a.direction)?;
            let mut store = DelegationStore::load(&paths.delegation_links_file)?;
            let mut link = DelegationLink::new(&a.name, &a.endpoint, direction);
            link.max_concurrency = a.concurrency;
            link.allow_tasks = a.allow;
            link.deny_tasks = a.deny;
            store.add_link(link);
            store.save(&paths.delegation_links_file)?;
            Ok(format!("delegation: link '{}' added", a.name))
        }
        DelegationCommands::Remove(a) => {
            let mut store = DelegationStore::load(&paths.delegation_links_file)?;
            match store.remove_link(&a.name) {
                Some(_) => {
                    store.save(&paths.delegation_links_file)?;
                    Ok(format!("delegation: link '{}' removed", a.name))
                }
                None => Ok(format!("delegation: link '{}' not found", a.name)),
            }
        }
        DelegationCommands::Enable(a) => toggle_link(&paths, &a.name, true),
        DelegationCommands::Disable(a) => toggle_link(&paths, &a.name, false),
    }
}

fn toggle_link(paths: &PraxisPaths, name: &str, enabled: bool) -> Result<String> {
    let mut store = DelegationStore::load(&paths.delegation_links_file)?;
    match store.links.get_mut(name) {
        Some(link) => {
            link.enabled = enabled;
            store.save(&paths.delegation_links_file)?;
            let state = if enabled { "enabled" } else { "disabled" };
            Ok(format!("delegation: link '{name}' {state}"))
        }
        None => Ok(format!("delegation: link '{name}' not found")),
    }
}

fn parse_direction(s: &str) -> Result<LinkDirection> {
    match s.to_ascii_lowercase().as_str() {
        "outbound" | "out" => Ok(LinkDirection::Outbound),
        "inbound" | "in" => Ok(LinkDirection::Inbound),
        "bidirectional" | "both" => Ok(LinkDirection::Bidirectional),
        other => {
            anyhow::bail!("unknown direction '{other}'; use outbound, inbound, or bidirectional")
        }
    }
}
