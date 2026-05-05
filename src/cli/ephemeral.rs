use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::{
    config::AppConfig,
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct EphemeralArgs {
    #[command(subcommand)]
    command: EphemeralCommand,
}

#[derive(Debug, Subcommand)]
enum EphemeralCommand {
    /// Set an ephemeral prompt for a channel.
    Set(SetEphemeralArgs),
    /// List all configured ephemeral prompts.
    List,
    /// Remove the ephemeral prompt for a channel.
    Remove(RemoveEphemeralArgs),
}

#[derive(Debug, Args)]
struct SetEphemeralArgs {
    /// Channel identifier (e.g. Telegram chat ID).
    channel_id: String,
    /// The ephemeral prompt text to inject for this channel.
    prompt: String,
}

#[derive(Debug, Args)]
struct RemoveEphemeralArgs {
    /// Channel identifier to remove the prompt for.
    channel_id: String,
}

pub(super) fn handle_ephemeral(
    data_dir_override: Option<PathBuf>,
    args: EphemeralArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        EphemeralCommand::Set(a) => {
            let mut config = load_config(&paths)?;
            config.ephemeral_prompts.insert(a.channel_id.clone(), a.prompt);
            save_config(&paths, &config)?;
            Ok(format!("ephemeral: prompt set for channel '{}'", a.channel_id))
        }

        EphemeralCommand::List => {
            let config = load_config(&paths)?;
            if config.ephemeral_prompts.is_empty() {
                return Ok("ephemeral: no prompts configured".to_string());
            }
            let mut lines = Vec::new();
            let mut sorted: Vec<_> = config.ephemeral_prompts.iter().collect();
            sorted.sort_by_key(|(k, _)| (*k).clone());
            for (channel_id, prompt) in sorted {
                let preview = if prompt.len() > 80 {
                    format!("{}…", &prompt[..80])
                } else {
                    prompt.clone()
                };
                lines.push(format!("{channel_id}: {preview}"));
            }
            Ok(lines.join("\n"))
        }

        EphemeralCommand::Remove(a) => {
            let mut config = load_config(&paths)?;
            if config.ephemeral_prompts.remove(&a.channel_id).is_some() {
                save_config(&paths, &config)?;
                Ok(format!("ephemeral: prompt removed for channel '{}'", a.channel_id))
            } else {
                Ok(format!("ephemeral: no prompt found for channel '{}'", a.channel_id))
            }
        }
    }
}

fn load_config(paths: &PraxisPaths) -> Result<AppConfig> {
    AppConfig::load(&paths.config_file)
        .with_context(|| format!("failed to load {}", paths.config_file.display()))
}

fn save_config(paths: &PraxisPaths, config: &AppConfig) -> Result<()> {
    config.save(&paths.config_file)
}
