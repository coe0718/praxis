/// WhatsApp messaging platform adapter.
///
/// WhatsApp Cloud API (Meta) provides a REST API for sending messages.
/// Set `PRAXIS_WHATSAPP_PHONE_ID` and `PRAXIS_WHATSAPP_ACCESS_TOKEN`.
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct WhatsAppClient {
    client: Client,
    phone_id: String,
    access_token: String,
}

#[derive(Debug, Serialize)]
struct WhatsAppMessageRequest {
    messaging_product: String,
    to: String,
    text: WhatsAppText,
}

#[derive(Debug, Serialize)]
struct WhatsAppText {
    body: String,
}

#[derive(Debug, Serialize)]
struct WhatsAppMediaRequest {
    messaging_product: String,
    to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppResponse {
    messages: Option<Vec<WhatsAppMessage>>,
}

#[derive(Debug, Deserialize)]
struct WhatsAppMessage {
    id: String,
}

#[derive(Debug, Deserialize)]
struct WhatsAppError {
    message: String,
    #[serde(rename = "error_user_title")]
    error_user_title: Option<String>,
}

impl WhatsAppClient {
    pub fn from_env() -> Result<Self> {
        let phone_id = std::env::var("PRAXIS_WHATSAPP_PHONE_ID")
            .context("PRAXIS_WHATSAPP_PHONE_ID is required for WhatsApp")?;
        let access_token = std::env::var("PRAXIS_WHATSAPP_ACCESS_TOKEN")
            .context("PRAXIS_WHATSAPP_ACCESS_TOKEN is required for WhatsApp")?;

        Ok(Self {
            client: Client::new(),
            phone_id,
            access_token,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_id = std::env::var("PRAXIS_WHATSAPP_PHONE_ID").is_ok();
        let has_token = std::env::var("PRAXIS_WHATSAPP_ACCESS_TOKEN").is_ok();
        if !has_id || !has_token {
            bail!("PRAXIS_WHATSAPP_PHONE_ID and PRAXIS_WHATSAPP_ACCESS_TOKEN are required for WhatsApp");
        }
        Ok(())
    }
}

impl crate::messaging::Platform for WhatsAppClient {
    fn name(&self) -> &str {
        "whatsapp"
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn send_message(&self, target: &str, text: &str) -> Result<()> {
        let request = WhatsAppMessageRequest {
            messaging_product: "whatsapp".to_string(),
            to: target.to_string(),
            text: WhatsAppText {
                body: text.to_string(),
            },
        };

        let url = format!(
            "https://graph.facebook.com/v19.0/{}/messages",
            self.phone_id
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .context("failed to send WhatsApp message")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("WhatsApp send failed with {status}: {body}");
        }

        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        let request = WhatsAppMediaRequest {
            messaging_product: "whatsapp".to_string(),
            to: target.to_string(),
            caption: caption.map(str::to_string),
        };

        let url = format!(
            "https://graph.facebook.com/v19.0/{}/media",
            self.phone_id
        );

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&request)
            .send()
            .context("failed to send WhatsApp media")?;

        if !response.status().is_success() {
            let status = response.status();
            bail!("WhatsApp media send failed with {status}");
        }

        Ok(())
    }

    fn send_typing(&self, _target: &str) -> Result<()> {
        // WhatsApp doesn't have a typing indicator API - no-op
        Ok(())
    }
}
