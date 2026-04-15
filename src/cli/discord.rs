use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::messaging::DiscordClient;

#[derive(Debug, Args)]
pub struct DiscordArgs {
    #[command(subcommand)]
    command: DiscordCommand,
}

#[derive(Debug, Subcommand)]
enum DiscordCommand {
    /// Check Discord environment variables are configured.
    Doctor,
    /// Send a message via the configured webhook.
    Send(DiscordSendArgs),
    /// Send a message to a channel via the Bot REST API.
    Post(DiscordPostArgs),
}

#[derive(Debug, Args)]
struct DiscordSendArgs {
    /// Message text.
    #[arg(required = true)]
    text: Vec<String>,

    /// Override the webhook username.
    #[arg(long)]
    username: Option<String>,
}

#[derive(Debug, Args)]
struct DiscordPostArgs {
    /// Target channel ID.
    #[arg(long)]
    channel_id: String,

    /// Message text.
    #[arg(required = true)]
    text: Vec<String>,
}

pub(crate) fn handle_discord(
    _data_dir_override: Option<PathBuf>,
    args: DiscordArgs,
) -> Result<String> {
    match args.command {
        DiscordCommand::Doctor => {
            DiscordClient::validate_environment()?;
            Ok("discord: ok".to_string())
        }
        DiscordCommand::Send(a) => {
            let text = a.text.join(" ");
            let client = DiscordClient::from_env()?;
            client.send_webhook(&text, a.username.as_deref())?;
            Ok(format!("discord: sent via webhook"))
        }
        DiscordCommand::Post(a) => {
            let text = a.text.join(" ");
            let client = DiscordClient::from_env()?;
            let msg = client.send_message(&a.channel_id, &text)?;
            Ok(format!("discord: posted message {}", msg.id))
        }
    }
}
