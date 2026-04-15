pub mod activation;
pub mod context_group;
#[cfg(feature = "discord")]
pub mod discord;
pub mod pairing;
mod router;
#[cfg(feature = "slack")]
pub mod slack;
mod telegram;
pub mod typing;

pub use activation::{ActivationMode, ActivationStore};
#[cfg(feature = "discord")]
pub use discord::DiscordClient;
pub use router::{handle_telegram_command, parse_telegram_command};
#[cfg(feature = "slack")]
pub use slack::SlackClient;
pub use telegram::{TelegramBot, TelegramMessage, TelegramUpdate};
pub use typing::{NoopTypingIndicator, TypingIndicator};
