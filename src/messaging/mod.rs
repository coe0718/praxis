pub mod activation;
pub mod auto_reply;
pub mod context_group;
#[cfg(feature = "discord")]
pub mod discord;
#[cfg(feature = "discord")]
mod discord_gateway;
pub mod inbound;
pub mod pairing;
pub mod platform;
mod router;
#[cfg(feature = "slack")]
pub mod slack;
mod telegram;
pub mod typing;

pub use activation::{ActivationMode, ActivationStore};
pub use auto_reply::{AutoReplyConfig, AutoReplyEngine};
#[cfg(feature = "discord")]
pub use discord::{
    DiscordClient, DiscordPollMessage, parse_allowed_user_ids as discord_allowed_user_ids,
};
#[cfg(feature = "discord")]
pub use discord_gateway::run_gateway;
pub use inbound::{InboundPollConfig, poll_discord_messages, poll_slack_messages};
pub use platform::{Platform, PlatformRegistry};
pub use router::{
    handle_discord_command, handle_slack_command, handle_telegram_command, parse_telegram_command,
};
#[cfg(feature = "slack")]
pub use slack::{SlackClient, SlackPollMessage, parse_allowed_user_ids as slack_allowed_user_ids};
pub use telegram::{MessageGating, TelegramBot, TelegramMessage, TelegramUpdate};
pub use typing::{NoopTypingIndicator, TypingIndicator};
