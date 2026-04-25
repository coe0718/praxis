use std::{path::PathBuf, thread, time::Duration};

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::messaging::{SlackClient, handle_slack_command, slack_allowed_user_ids};

use super::core::load_initialized_config;

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
    /// Poll watched channels once and process any new messages.
    PollOnce,
    /// Continuously poll watched channels and process new messages.
    Run(SlackRunArgs),
}

#[derive(Debug, Args)]
struct SlackSendArgs {
    #[arg(required = true)]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct SlackPostArgs {
    #[arg(long)]
    channel: String,
    #[arg(required = true)]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct SlackRunArgs {
    #[arg(long, default_value_t = 0)]
    cycles: u32,
    #[arg(long, default_value_t = 2_000)]
    sleep_ms: u64,
}

pub(crate) fn handle_slack(data_dir_override: Option<PathBuf>, args: SlackArgs) -> Result<String> {
    match args.command {
        SlackCommand::Doctor => {
            SlackClient::validate_environment()?;
            Ok("slack: ok".to_string())
        }
        SlackCommand::Send(a) => {
            let text = a.text.join(" ");
            let client = SlackClient::from_env()?;
            client.send_webhook(&text)?;
            Ok("slack: sent via webhook".to_string())
        }
        SlackCommand::Post(a) => {
            let text = a.text.join(" ");
            let client = SlackClient::from_env()?;
            let ts = client.post_message(&a.channel, &text)?;
            Ok(format!("slack: posted message ts={ts}"))
        }
        SlackCommand::PollOnce => poll_once(data_dir_override),
        SlackCommand::Run(a) => poll_loop(data_dir_override, a),
    }
}

fn poll_once(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let client = SlackClient::from_env()?;
    let allowed = slack_allowed_user_ids();
    let messages = client.poll_once(&paths.slack_state_file, &allowed)?;
    let processed = process_messages(&client, &paths, messages)?;
    Ok(format!("slack: processed {processed}"))
}

fn poll_loop(data_dir_override: Option<PathBuf>, args: SlackRunArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let client = SlackClient::from_env()?;
    let allowed = slack_allowed_user_ids();
    let delay = Duration::from_millis(args.sleep_ms.max(500));
    let mut cycles = 0_u32;
    let mut total = 0_usize;

    loop {
        let messages = client.poll_once(&paths.slack_state_file, &allowed)?;
        total += process_messages(&client, &paths, messages)?;
        cycles += 1;
        if args.cycles > 0 && cycles >= args.cycles {
            break;
        }
        thread::sleep(delay);
    }

    Ok(format!("slack: processed {total} messages across {cycles} cycles"))
}

fn process_messages(
    client: &SlackClient,
    paths: &crate::paths::PraxisPaths,
    messages: Vec<crate::messaging::SlackPollMessage>,
) -> Result<usize> {
    for msg in &messages {
        let reply = handle_slack_command(paths.data_dir.clone(), &msg.channel_id, &msg.text)
            .unwrap_or_else(|e| format!("slack command error: {e}"));
        if let Err(e) = client.post_message(&msg.channel_id, &reply) {
            log::warn!("slack reply failed for channel {}: {e}", msg.channel_id);
        }
    }
    Ok(messages.len())
}
