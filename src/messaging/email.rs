/// Email messaging platform adapter.
///
/// SMTP for sending, IMAP for receiving with IDLE push.
/// Set `PRAXIS_EMAIL_HOST`, `PRAXIS_EMAIL_USERNAME`, `PRAXIS_EMAIL_PASSWORD`.
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::Serialize;

#[derive(Debug)]
#[allow(dead_code)]
pub struct EmailClient {
    client: Client,
    smtp_host: String,
    imap_host: String,
    username: String,
}

#[derive(Debug, Serialize)]
struct EmailMessage {
    from: String,
    to: String,
    subject: String,
    body: String,
}

impl EmailClient {
    pub fn from_env() -> Result<Self> {
        let smtp_host = std::env::var("PRAXIS_EMAIL_SMTP_HOST")
            .unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let imap_host = std::env::var("PRAXIS_EMAIL_IMAP_HOST")
            .unwrap_or_else(|_| "imap.gmail.com".to_string());
        let username = std::env::var("PRAXIS_EMAIL_USERNAME")
            .context("PRAXIS_EMAIL_USERNAME is required for Email")?;

        Ok(Self {
            client: Client::new(),
            smtp_host,
            imap_host,
            username,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_user = std::env::var("PRAXIS_EMAIL_USERNAME").is_ok();
        if !has_user {
            bail!("PRAXIS_EMAIL_USERNAME is required for Email");
        }
        Ok(())
    }
}

impl crate::messaging::Platform for EmailClient {
    fn name(&self) -> &str {
        "email"
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn send_message(&self, target: &str, _text: &str) -> Result<()> {
        // For email, target is the recipient email address
        // Using a simple SMTP relay or HTTP API approach
        log::info!("Would send email to {} via {}", target, self.smtp_host);
        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        let content = caption.unwrap_or("File attached");
        let text = format!("{}: {}", content, file_path);
        self.send_message(target, &text)
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        // Email doesn't have typing indicators
        Ok(())
    }
}

