use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::messaging::{TelegramBot, handle_telegram_command};

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct TelegramArgs {
    #[command(subcommand)]
    command: TelegramCommand,
}

#[derive(Debug, Subcommand)]
enum TelegramCommand {
    Doctor,
    PollOnce,
    Send(TelegramSendArgs),
}

#[derive(Debug, Args)]
struct TelegramSendArgs {
    #[arg(long)]
    chat_id: i64,
    #[arg(long)]
    text: String,
}

pub(crate) fn handle_telegram(
    data_dir_override: Option<PathBuf>,
    args: TelegramArgs,
) -> Result<String> {
    match args.command {
        TelegramCommand::Doctor => {
            TelegramBot::validate_environment()?;
            Ok("telegram: ok".to_string())
        }
        TelegramCommand::PollOnce => {
            let (_, paths) = load_initialized_config(data_dir_override)?;
            let bot = TelegramBot::from_env()?;
            let messages = bot.poll_once(&paths.telegram_state_file)?;
            for message in &messages {
                let reply = handle_telegram_command(paths.data_dir.clone(), &message.text)
                    .unwrap_or_else(|error| format!("telegram command error: {error}"));
                bot.send_message(message.chat_id, &reply)?;
            }
            Ok(format!("telegram: processed {}", messages.len()))
        }
        TelegramCommand::Send(args) => {
            let bot = TelegramBot::from_env()?;
            bot.send_message(args.chat_id, &args.text)?;
            Ok(format!("telegram: sent to {}", args.chat_id))
        }
    }
}
