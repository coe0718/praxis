use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::{
    config::AppConfig,
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct FallbackArgs {
    #[command(subcommand)]
    pub command: FallbackCommand,
}

#[derive(Debug, Subcommand)]
pub enum FallbackCommand {
    /// Show the current fallback provider chain.
    List,
    /// Add a provider to the fallback chain.
    Add(FallbackAddArgs),
    /// Remove a provider from the fallback chain.
    Remove(FallbackRemoveArgs),
    /// Set an explicit fallback order (replaces the current chain).
    Reorder(FallbackReorderArgs),
}

#[derive(Debug, Args)]
pub struct FallbackAddArgs {
    /// Provider name to add as a fallback (e.g. "ollama", "openai").
    pub provider: String,

    /// Insert after this provider in the chain.  Omit to append at the end.
    #[arg(long)]
    pub after: Option<String>,
}

#[derive(Debug, Args)]
pub struct FallbackRemoveArgs {
    /// Provider name to remove from the fallback chain.
    pub provider: String,
}

#[derive(Debug, Args)]
pub struct FallbackReorderArgs {
    /// Comma-separated list of provider names in desired order
    /// (e.g. "ollama,openai,anthropic").
    pub providers: String,
}

pub fn handle_fallback(data_dir_override: Option<PathBuf>, args: FallbackArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        anyhow::bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    match args.command {
        FallbackCommand::List => handle_list(&paths),
        FallbackCommand::Add(a) => handle_add(&paths, a),
        FallbackCommand::Remove(a) => handle_remove(&paths, a),
        FallbackCommand::Reorder(a) => handle_reorder(&paths, a),
    }
}

fn load_config(paths: &PraxisPaths) -> Result<AppConfig> {
    AppConfig::load(&paths.config_file)
        .with_context(|| format!("failed to load {}", paths.config_file.display()))
}

fn save_config(paths: &PraxisPaths, config: &AppConfig) -> Result<()> {
    config
        .save(&paths.config_file)
        .with_context(|| format!("failed to save {}", paths.config_file.display()))
}

fn handle_list(paths: &PraxisPaths) -> Result<String> {
    let config = load_config(paths)?;
    let primary = &config.agent.backend;
    let chain = &config.agent.fallback_providers;

    if chain.is_empty() {
        return Ok(format!("primary: {primary}\nfallback chain: (empty)"));
    }

    let mut lines = vec![format!("primary: {primary}"), "fallback chain:".to_string()];
    for (i, provider) in chain.iter().enumerate() {
        lines.push(format!("  {}. {provider}", i + 1));
    }
    Ok(lines.join("\n"))
}

fn handle_add(paths: &PraxisPaths, args: FallbackAddArgs) -> Result<String> {
    let mut config = load_config(paths)?;
    let chain = &mut config.agent.fallback_providers;

    if chain.contains(&args.provider) {
        anyhow::bail!("'{}' is already in the fallback chain", args.provider);
    }

    match args.after {
        Some(ref anchor) => {
            let pos = chain
                .iter()
                .position(|p| p == anchor)
                .with_context(|| format!("provider '{}' not found in fallback chain", anchor))?;
            chain.insert(pos + 1, args.provider.clone());
        }
        None => {
            chain.push(args.provider.clone());
        }
    }

    save_config(paths, &config)?;
    Ok(format!(
        "added '{}' to fallback chain (position {})",
        args.provider,
        config
            .agent
            .fallback_providers
            .iter()
            .position(|p| p == &args.provider)
            .map(|i| i + 1)
            .unwrap_or(chain.len())
    ))
}

fn handle_remove(paths: &PraxisPaths, args: FallbackRemoveArgs) -> Result<String> {
    let mut config = load_config(paths)?;
    let chain = &mut config.agent.fallback_providers;

    let pos = chain
        .iter()
        .position(|p| p == &args.provider)
        .with_context(|| format!("'{}' not found in fallback chain", args.provider))?;

    chain.remove(pos);
    save_config(paths, &config)?;
    Ok(format!("removed '{}' from fallback chain", args.provider))
}

fn handle_reorder(paths: &PraxisPaths, args: FallbackReorderArgs) -> Result<String> {
    let mut config = load_config(paths)?;

    let new_chain: Vec<String> = args
        .providers
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if new_chain.is_empty() {
        anyhow::bail!("provider list must not be empty");
    }

    // Check for duplicates
    let mut seen = std::collections::HashSet::new();
    for provider in &new_chain {
        if !seen.insert(provider.as_str()) {
            anyhow::bail!("duplicate provider '{}' in reorder list", provider);
        }
    }

    config.agent.fallback_providers = new_chain;
    save_config(paths, &config)?;

    let mut lines = vec!["fallback chain reordered:".to_string()];
    for (i, provider) in config.agent.fallback_providers.iter().enumerate() {
        lines.push(format!("  {}. {provider}", i + 1));
    }
    Ok(lines.join("\n"))
}
