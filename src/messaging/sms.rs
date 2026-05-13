/// SMS messaging platform adapter.
///
/// Twilio/SNS for sending SMS messages.
/// Set `PRAXIS_SMS_ACCOUNT_SID` and `PRAXIS_SMS_AUTH_TOKEN`.
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::Serialize;

#[derive(Debug)]
pub struct SmsClient {
    client: Client,
    account_sid: String,
    auth_token: String,
    from_number: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct SmsMessage {
    to: String,
    from: String,
    body: String,
}

impl SmsClient {
    pub fn from_env() -> Result<Self> {
        let account_sid = std::env::var("PRAXIS_SMS_ACCOUNT_SID")
            .context("PRAXIS_SMS_ACCOUNT_SID is required for SMS")?;
        let auth_token = std::env::var("PRAXIS_SMS_AUTH_TOKEN")
            .context("PRAXIS_SMS_AUTH_TOKEN is required for SMS")?;
        let from_number = std::env::var("PRAXIS_SMS_FROM_NUMBER")
            .unwrap_or_else(|_| "+14155238886".to_string()); // Twilio default

        Ok(Self {
            client: Client::new(),
            account_sid,
            auth_token,
            from_number,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_sid = std::env::var("PRAXIS_SMS_ACCOUNT_SID").is_ok();
        let has_token = std::env::var("PRAXIS_SMS_AUTH_TOKEN").is_ok();
        if !has_sid || !has_token {
            bail!("PRAXIS_SMS_ACCOUNT_SID and PRAXIS_SMS_AUTH_TOKEN are required for SMS");
        }
        Ok(())
    }
}

impl crate::messaging::Platform for SmsClient {
    fn name(&self) -> &str {
        "sms"
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn send_message(&self, target: &str, text: &str) -> Result<()> {
        // Twilio API endpoint
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&[
                ("To", target),
                ("From", &self.from_number),
                ("Body", text),
            ])
            .send()
            .context("failed to send SMS")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("SMS send failed with {status}: {body}");
        }

        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        let text = match caption {
            Some(c) => format!("{}: {}", c, file_path),
            None => format!("File: {}", file_path),
        };
        self.send_message(target, &text)
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        // SMS doesn't have typing indicators
        Ok(())
    }
}
