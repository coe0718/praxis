use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::messaging::SlackClient;

#[derive(Debug, Args)]
pub struct SlackArgs {
    #[command(subcommand)]
    command: SlackCommand,
}

#[derive(Debug, Subcommand)]
enum SlackCommand {
    /// Check Slack environment variables are configured.
    Doctor,
    /// Send a message via the configured incoming webhook.
    Send(SlackSendArgs),
    /// Post a message to a channel via the Web API (requires bot token).
    Post(SlackPostArgs),
}

#[derive(Debug, Args)]
struct SlackSendArgs {
    /// Message text.
    #[arg(required = true)]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct SlackPostArgs {
    /// Target channel ID or name (e.g. #general or C01234ABCD).
    #[arg(long)]
    channel: String,

    /// Message text.
    #[arg(required = true)]
    text: Vec<String>,
}

pub(crate) fn handle_slack(_data_dir_override: Option<PathBuf>, args: SlackArgs) -> Result<String> {
    match args.command {
        SlackCommand::Doctor => {
            SlackClient::validate_environment()?;
            Ok("slack: ok".to_string())
        }
        SlackCommand::Send(a) => {
            let text = a.text.join(" ");
            let client = SlackClient::from_env()?;
            client.send_webhook(&text)?;
            Ok(format!("slack: sent via webhook"))
        }
        SlackCommand::Post(a) => {
            let text = a.text.join(" ");
            let client = SlackClient::from_env()?;
            let ts = client.post_message(&a.channel, &text)?;
            Ok(format!("slack: posted message ts={ts}"))
        }
    }
}
