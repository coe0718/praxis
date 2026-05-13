/// Signal messaging platform adapter.
///
/// Signal CLI provides a JSON-RPC interface for sending messages.
/// Set `PRAXIS_SIGNAL_PHONE_NUMBER` for the account's phone number.
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[allow(dead_code)]
pub struct SignalClient {
    client: Client,
    phone_number: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct SignalMessageRequest {
    message: String,
    recipient: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<String>,
}

impl SignalClient {
    pub fn from_env() -> Result<Self> {
        let phone_number = std::env::var("PRAXIS_SIGNAL_PHONE_NUMBER")
            .context("PRAXIS_SIGNAL_PHONE_NUMBER is required for Signal")?;

        Ok(Self {
            client: Client::new(),
            phone_number,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_phone = std::env::var("PRAXIS_SIGNAL_PHONE_NUMBER").is_ok();
        if !has_phone {
            bail!("PRAXIS_SIGNAL_PHONE_NUMBER is required for Signal");
        }
        Ok(())
    }
}

impl crate::messaging::Platform for SignalClient {
    fn name(&self) -> &str {
        "signal"
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn send_message(&self, target: &str, text: &str) -> Result<()> {
        let request = SignalMessageRequest {
            message: text.to_string(),
            recipient: target.to_string(),
            attachment: None,
        };

        let response = self
            .client
            .post("http://localhost:8080/v1/messages")
            .json(&request)
            .send()
            .context("failed to send Signal message")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Signal send failed with {status}: {body}");
        }

        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        let request = SignalMessageRequest {
            message: caption.unwrap_or("").to_string(),
            recipient: target.to_string(),
            attachment: Some(file_path.to_string()),
        };

        let response = self
            .client
            .post("http://localhost:8080/v1/messages")
            .json(&request)
            .send()
            .context("failed to send Signal file")?;

        if !response.status().is_success() {
            let status = response.status();
            bail!("Signal file send failed with {status}");
        }

        Ok(())
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        Ok(())
    }
}

/// Inbound message from Signal (via webhook or polling).
#[derive(Debug, Clone, Deserialize)]
pub struct SignalUpdate {
    pub envelope: SignalEnvelope,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignalEnvelope {
    pub source: String,
    pub data_message: Option<SignalDataMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SignalDataMessage {
    pub body: Option<String>,
}