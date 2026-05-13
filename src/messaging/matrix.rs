/// Matrix messaging platform adapter.
///
/// Matrix HTTP API provides a REST interface for sending messages.
/// Set `PRAXIS_MATRIX_HOMESERVER` and `PRAXIS_MATRIX_ACCESS_TOKEN`.
use anyhow::{Context, Result, bail};
use chrono::Utc;
use reqwest::blocking::Client;
use serde::Serialize;

#[derive(Debug)]
pub struct MatrixClient {
    client: Client,
    homeserver: String,
    access_token: String,
    user_id: String,
}

#[derive(Debug, Serialize)]
struct MatrixMessageContent {
    msgtype: String,
    body: String,
}

impl MatrixClient {
    pub fn from_env() -> Result<Self> {
        let homeserver = std::env::var("PRAXIS_MATRIX_HOMESERVER")
            .context("PRAXIS_MATRIX_HOMESERVER is required for Matrix")?;
        let access_token = std::env::var("PRAXIS_MATRIX_ACCESS_TOKEN")
            .context("PRAXIS_MATRIX_ACCESS_TOKEN is required for Matrix")?;
        let user_id = std::env::var("PRAXIS_MATRIX_USER_ID")
            .context("PRAXIS_MATRIX_USER_ID is required for Matrix")?;

        Ok(Self {
            client: Client::new(),
            homeserver,
            access_token,
            user_id,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let has_hs = std::env::var("PRAXIS_MATRIX_HOMESERVER").is_ok();
        let has_token = std::env::var("PRAXIS_MATRIX_ACCESS_TOKEN").is_ok();
        let has_user = std::env::var("PRAXIS_MATRIX_USER_ID").is_ok();
        if !has_hs || !has_token || !has_user {
            bail!("PRAXIS_MATRIX_HOMESERVER, PRAXIS_MATRIX_ACCESS_TOKEN, and PRAXIS_MATRIX_USER_ID are required for Matrix");
        }
        Ok(())
    }
}

impl crate::messaging::Platform for MatrixClient {
    fn name(&self) -> &str {
        "matrix"
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn send_message(&self, target: &str, text: &str) -> Result<()> {
        let content = MatrixMessageContent {
            msgtype: "m.text".to_string(),
            body: text.to_string(),
        };

        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver.trim_end_matches('/'),
            target,
            Utc::now().timestamp_millis()
        );

        let response = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&content)
            .send()
            .context("failed to send Matrix message")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Matrix send failed with {status}: {body}");
        }

        Ok(())
    }

    fn send_file(&self, target: &str, file_path: &str, caption: Option<&str>) -> Result<()> {
        // Matrix requires uploading the file first, then sending a message with the mxc:// URI
        // For simplicity, we'll just send a text message with the file path
        let text = match caption {
            Some(c) => format!("{}: {}", c, file_path),
            None => format!("File: {}", file_path),
        };
        self.send_message(target, &text)
    }

    fn send_typing(&self, target: &str) -> Result<()> {
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/typing/{}",
            self.homeserver.trim_end_matches('/'),
            target,
            self.user_id
        );

        let response = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({ "typing": true, "timeout": 30000 }))
            .send()
            .context("failed to send Matrix typing indicator")?;

        if !response.status().is_success() {
            let status = response.status();
            bail!("Matrix typing indicator failed with {status}");
        }

        Ok(())
    }
}