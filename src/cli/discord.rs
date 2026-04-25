use std::{path::PathBuf, thread, time::Duration};

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::messaging::{DiscordClient, discord_allowed_user_ids, handle_discord_command};

use super::core::load_initialized_config;

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
    /// Poll watched channels once and process any new messages.
    PollOnce,
    /// Continuously poll watched channels and process new messages.
    Run(DiscordRunArgs),
}

#[derive(Debug, Args)]
struct DiscordSendArgs {
    #[arg(required = true)]
    text: Vec<String>,
    #[arg(long)]
    username: Option<String>,
}

#[derive(Debug, Args)]
struct DiscordPostArgs {
    #[arg(long)]
    channel_id: String,
    #[arg(required = true)]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct DiscordRunArgs {
    /// How many poll cycles to run (0 = run forever).
    #[arg(long, default_value_t = 0)]
    cycles: u32,
    /// Milliseconds between poll cycles.
    #[arg(long, default_value_t = 2_000)]
    sleep_ms: u64,
}

pub(crate) fn handle_discord(
    data_dir_override: Option<PathBuf>,
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
            Ok("discord: sent via webhook".to_string())
        }
        DiscordCommand::Post(a) => {
            let text = a.text.join(" ");
            let client = DiscordClient::from_env()?;
            let msg = client.send_message(&a.channel_id, &text)?;
            Ok(format!("discord: posted message {}", msg.id))
        }
        DiscordCommand::PollOnce => poll_once(data_dir_override),
        DiscordCommand::Run(a) => poll_loop(data_dir_override, a),
    }
}

fn poll_once(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let client = DiscordClient::from_env()?;
    let allowed = discord_allowed_user_ids();
    let messages = client.poll_once(&paths.discord_state_file, &allowed)?;
    let processed = process_messages(&client, &paths, messages)?;
    Ok(format!("discord: processed {processed}"))
}

fn poll_loop(data_dir_override: Option<PathBuf>, args: DiscordRunArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let client = DiscordClient::from_env()?;
    let allowed = discord_allowed_user_ids();
    let delay = Duration::from_millis(args.sleep_ms.max(500));
    let mut cycles = 0_u32;
    let mut total = 0_usize;

    loop {
        let messages = client.poll_once(&paths.discord_state_file, &allowed)?;
        total += process_messages(&client, &paths, messages)?;
        cycles += 1;
        if args.cycles > 0 && cycles >= args.cycles {
            break;
        }
        thread::sleep(delay);
    }

    Ok(format!("discord: processed {total} messages across {cycles} cycles"))
}

fn process_messages(
    client: &DiscordClient,
    paths: &crate::paths::PraxisPaths,
    messages: Vec<crate::messaging::DiscordPollMessage>,
) -> Result<usize> {
    for msg in &messages {
        let reply = handle_discord_command(paths.data_dir.clone(), &msg.channel_id, &msg.content)
            .unwrap_or_else(|e| format!("discord command error: {e}"));
        if let Err(e) = client.send_message(&msg.channel_id, &reply) {
            log::warn!("discord reply failed for channel {}: {e}", msg.channel_id);
        }
    }
    Ok(messages.len())
}
