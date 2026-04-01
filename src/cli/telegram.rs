use std::{path::PathBuf, thread, time::Duration};

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
    Run(TelegramRunArgs),
    Send(TelegramSendArgs),
}

#[derive(Debug, Args)]
struct TelegramSendArgs {
    #[arg(long)]
    chat_id: i64,
    #[arg(long)]
    text: String,
}

#[derive(Debug, Args)]
struct TelegramRunArgs {
    #[arg(long, default_value_t = 0)]
    cycles: u32,
    #[arg(long, default_value_t = 1_500)]
    sleep_ms: u64,
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
        TelegramCommand::PollOnce => run_poll_cycle(data_dir_override),
        TelegramCommand::Run(args) => run_poll_loop(data_dir_override, args),
        TelegramCommand::Send(args) => {
            let bot = TelegramBot::from_env()?;
            bot.send_message(args.chat_id, &args.text)?;
            Ok(format!("telegram: sent to {}", args.chat_id))
        }
    }
}

fn run_poll_cycle(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let bot = TelegramBot::from_env()?;
    let processed = process_messages(&bot, &paths)?;
    Ok(format!("telegram: processed {processed}"))
}

fn run_poll_loop(data_dir_override: Option<PathBuf>, args: TelegramRunArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let bot = TelegramBot::from_env()?;
    let delay = Duration::from_millis(args.sleep_ms.max(250));
    let mut cycles = 0_u32;
    let mut processed_total = 0_usize;

    loop {
        processed_total += process_messages(&bot, &paths)?;
        cycles += 1;
        if args.cycles > 0 && cycles >= args.cycles {
            break;
        }
        thread::sleep(delay);
    }

    Ok(format!(
        "telegram: processed {processed_total} messages across {cycles} cycles"
    ))
}

fn process_messages(bot: &TelegramBot, paths: &crate::paths::PraxisPaths) -> Result<usize> {
    let messages = bot.poll_once(&paths.telegram_state_file)?;
    for message in &messages {
        let reply = handle_telegram_command(paths.data_dir.clone(), &message.text)
            .unwrap_or_else(|error| format!("telegram command error: {error}"));
        bot.send_message(message.chat_id, &reply)?;
    }
    Ok(messages.len())
}
