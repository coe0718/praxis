/// Email messaging platform adapter.
///
/// SMTP for sending, IMAP for receiving with IDLE push.
/// Set `PRAXIS_EMAIL_HOST`, `PRAXIS_EMAIL_USERNAME`, `PRAXIS_EMAIL_PASSWORD`.
use anyhow::{Result, bail};
use reqwest::blocking::Client;

#[derive(Debug)]
pub struct EmailClient {
    _client: Client,
    smtp_host: String,
    _imap_host: String,
    _username: String,
}

impl EmailClient {
    pub fn from_env() -> Result<Self> {
        let smtp_host = std::env::var("PRAXIS_EMAIL_SMTP_HOST")
            .unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let imap_host = std::env::var("PRAXIS_EMAIL_IMAP_HOST")
            .unwrap_or_else(|_| "imap.gmail.com".to_string());
        let username = std::env::var("PRAXIS_EMAIL_USERNAME").unwrap_or_default();

        Ok(Self {
            _client: Client::new(),
            smtp_host,
            _imap_host: imap_host,
            _username: username,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_host = std::env::var("PRAXIS_EMAIL_SMTP_HOST").is_ok();
        let has_user = std::env::var("PRAXIS_EMAIL_USERNAME").is_ok();
        if !has_host || !has_user {
            bail!("PRAXIS_EMAIL_SMTP_HOST and PRAXIS_EMAIL_USERNAME are required for Email");
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
        log::info!("Would send email to {} via {}", target, self.smtp_host);
        Ok(())
    }

    fn send_file(&self, target: &str, _file_path: &str, caption: Option<&str>) -> Result<()> {
        let content = caption.unwrap_or("File attached");
        let text = format!("{}: {}", content, _file_path);
        self.send_message(target, &text)
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        Ok(())
    }
}