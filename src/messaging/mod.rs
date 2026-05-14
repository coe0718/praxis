pub mod activation;
pub mod auto_reply;
pub mod context_group;
#[cfg(feature = "discord")]
pub mod discord;
#[cfg(feature = "discord")]
mod discord_gateway;
pub mod email;
pub mod email_idle;
pub mod inbound;
pub mod matrix;
mod matrix_sync;
pub mod pairing;
pub mod platform;
mod router;
pub mod signal;
#[cfg(feature = "slack")]
pub mod slack;
pub mod slack_events;
pub mod sms;
mod telegram;
pub mod telegram_webhook;
pub mod typing;
pub mod whatsapp;
pub mod whatsapp_webhook;

pub use activation::{ActivationMode, ActivationStore};
pub use auto_reply::{AutoReplyConfig, AutoReplyEngine};
#[cfg(feature = "discord")]
pub use discord::{
    DiscordClient, DiscordPollMessage, parse_allowed_user_ids as discord_allowed_user_ids,
};
#[cfg(feature = "discord")]
pub use discord_gateway::run_gateway;
pub use email::EmailClient;
pub use email_idle::{ImapIdleConfig, run_imap_idle};
pub use inbound::{InboundPollConfig, poll_discord_messages, poll_slack_messages};
pub use matrix::MatrixClient;
pub use matrix_sync::{MatrixSync, MatrixSyncConfig};
pub use platform::{Platform, PlatformRegistry};
pub use router::{
    handle_discord_command, handle_slack_command, handle_telegram_command, parse_telegram_command,
};
pub use signal::SignalClient;
#[cfg(feature = "slack")]
pub use slack::{SlackClient, SlackPollMessage, parse_allowed_user_ids as slack_allowed_user_ids};
pub use slack_events::{
    SlackEventsConfig, handle_url_challenge, parse_event_payload, verify_slack_signature,
};
pub use sms::SmsClient;
pub use telegram::{MessageGating, TelegramBot, TelegramMessage, TelegramUpdate};
pub use telegram_webhook::{
    TelegramWebhookConfig, delete_telegram_webhook, get_webhook_info, set_telegram_webhook,
};
pub use typing::{NoopTypingIndicator, TypingIndicator};
pub use whatsapp::WhatsAppClient;
pub use whatsapp_webhook::{whatsapp_inbound, whatsapp_verify};
