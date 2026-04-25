//! CLI commands for managing dynamic webhook subscriptions.

use anyhow::Result;
use chrono::Utc;
use clap::{Args, Subcommand};

use crate::{
    paths::PraxisPaths,
    webhooks::{Webhook, WebhookStore},
};

#[derive(Debug, Args)]
pub struct WebhookArgs {
    #[command(subcommand)]
    pub command: WebhookCommand,
}

#[derive(Debug, Subcommand)]
pub enum WebhookCommand {
    /// Register a new webhook subscription.
    Subscribe(SubscribeArgs),
    /// List all registered webhooks.
    List,
    /// Remove a webhook subscription.
    Unsubscribe(UnsubscribeArgs),
}

#[derive(Debug, Args)]
pub struct SubscribeArgs {
    /// Unique name — also becomes the URL path: /webhook/{name}.
    pub name: String,
    /// Human-readable label.
    #[arg(long)]
    pub description: Option<String>,
    /// HMAC-SHA256 secret for signature verification. Omit for no-auth.
    #[arg(long)]
    pub secret: Option<String>,
    /// Comma-separated event types this webhook accepts.
    #[arg(long, default_value = "")]
    pub events: String,
}

#[derive(Debug, Args)]
pub struct UnsubscribeArgs {
    /// Name of the webhook to remove.
    pub name: String,
}

pub fn handle_webhook(
    data_dir_override: Option<std::path::PathBuf>,
    args: &WebhookArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(crate::paths::default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);
    let mut store = WebhookStore::load(&paths.webhooks_file)?;

    match &args.command {
        WebhookCommand::Subscribe(s) => {
            let wh = Webhook {
                name: s.name.clone(),
                description: s.description.clone().unwrap_or_else(|| s.name.clone()),
                secret: s.secret.clone(),
                events: s.events.clone(),
                created_at: Utc::now(),
                last_triggered_at: None,
                trigger_count: 0,
            };
            let was_update = store.get(&wh.name).is_some();
            store.upsert(wh.clone());
            store.save(&paths.webhooks_file)?;

            let action = if was_update { "Updated" } else { "Registered" };
            Ok(format!(
                "{action} webhook '{name}'. Endpoint: POST /webhook/{name}",
                name = wh.name
            ))
        }
        WebhookCommand::List => {
            if store.webhooks.is_empty() {
                return Ok("No webhooks registered.".to_string());
            }
            let lines: Vec<String> = store
                .webhooks
                .iter()
                .map(|w| {
                    let secret_status = if w.secret.is_some() {
                        "🔒 verified"
                    } else {
                        "⚠️ no auth"
                    };
                    let events = if w.events.is_empty() {
                        "all".to_string()
                    } else {
                        w.events.clone()
                    };
                    format!(
                        "  {name} ({secret_status}) — {desc} — events: {events} — triggers: {count}",
                        name = w.name,
                        desc = w.description,
                        count = w.trigger_count,
                    )
                })
                .collect();
            Ok(format!("Webhooks:\n{}", lines.join("\n")))
        }
        WebhookCommand::Unsubscribe(u) => {
            if store.remove(&u.name) {
                store.save(&paths.webhooks_file)?;
                Ok(format!("Removed webhook '{}'.", u.name))
            } else {
                Ok(format!("No webhook found with name '{}'.", u.name))
            }
        }
    }
}
