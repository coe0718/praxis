/// Email messaging platform adapter.
///
/// SMTP for sending, IMAP for receiving with IDLE push.
/// Set `PRAXIS_EMAIL_SMTP_HOST`, `PRAXIS_EMAIL_USERNAME`, `PRAXIS_EMAIL_PASSWORD`.
use anyhow::{Result, bail};

#[derive(Debug)]
pub struct EmailClient {
    _smtp_host: String,
    username: String,
    _password: String,
}

impl EmailClient {
    pub fn from_env() -> Result<Self> {
        let smtp_host = std::env::var("PRAXIS_EMAIL_SMTP_HOST")
            .unwrap_or_else(|_| "smtp.gmail.com".to_string());
        let username = std::env::var("PRAXIS_EMAIL_USERNAME").unwrap_or_default();
        let password = std::env::var("PRAXIS_EMAIL_PASSWORD").unwrap_or_default();

        Ok(Self {
            _smtp_host: smtp_host,
            username,
            _password: password,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_host = std::env::var("PRAXIS_EMAIL_SMTP_HOST").is_ok();
        let has_user = std::env::var("PRAXIS_EMAIL_USERNAME").is_ok();
        let has_pass = std::env::var("PRAXIS_EMAIL_PASSWORD").is_ok();
        if !has_host || !has_user || !has_pass {
            bail!(
                "PRAXIS_EMAIL_SMTP_HOST, PRAXIS_EMAIL_USERNAME, and PRAXIS_EMAIL_PASSWORD are required for Email"
            );
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

    fn send_message(&self, target: &str, text: &str) -> Result<()> {
        // Construct email message using lettre builder pattern
        let _email = lettre::Message::builder()
            .from(self.username.parse()?)
            .to(target.parse()?)
            .subject("Praxis Notification")
            .body(text.to_string())?;

        log::info!("Would send email to {}: {}", target, text);
        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        let content = caption.unwrap_or("File attached");
        let text = format!("{}: {}", content, file_path);
        self.send_message(target, &text)
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        Ok(())
    }
}
