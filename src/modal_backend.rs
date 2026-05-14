//! Modal Terminal Backend — Execute tools in Modal container sandboxes.
//!
//! Modal provides serverless container execution via a REST API.
//! Docs: <https://modal.com/docs>
//!
//! Set `PRAXIS_MODAL_API_KEY` and optionally `PRAXIS_MODAL_IMAGE`.
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::time::Duration;

/// Modal API configuration.
#[derive(Debug, Clone)]
pub struct ModalConfig {
    api_key: String,
    api_url: String,
    default_image: String,
    timeout: Duration,
}

impl ModalConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("PRAXIS_MODAL_API_KEY")
            .context("PRAXIS_MODAL_API_KEY is required for Modal backend")?;
        let api_url = std::env::var("PRAXIS_MODAL_API_URL")
            .unwrap_or_else(|_| "https://api.modal.com".to_string());
        let default_image =
            std::env::var("PRAXIS_MODAL_IMAGE").unwrap_or_else(|_| "ubuntu:22.04".to_string());
        let timeout_secs: u64 = std::env::var("PRAXIS_MODAL_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);

        Ok(Self {
            api_key,
            api_url,
            default_image,
            timeout: Duration::from_secs(timeout_secs),
        })
    }

    pub fn validate_environment() -> Result<()> {
        if std::env::var("PRAXIS_MODAL_API_KEY").is_err() {
            anyhow::bail!("PRAXIS_MODAL_API_KEY is required for Modal backend");
        }
        Ok(())
    }
}

/// A Modal container sandbox for executing commands.
pub struct ModalBackend {
    config: ModalConfig,
    client: Client,
}

impl ModalBackend {
    pub fn new(config: ModalConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { config, client }
    }

    /// Execute a command inside a Modal container.
    ///
    /// Creates a container, runs the command, captures stdout/stderr, and returns the output.
    pub fn execute(&self, command: &str, image: Option<&str>) -> Result<String> {
        let image = image.unwrap_or(&self.config.default_image);

        // Modal's container API: POST to create a container, then exec.
        // We use the Modal v2 API which manages ephemeral sandboxes.
        let sandbox_body = serde_json::json!({
            "image": image,
            "command": ["/bin/sh", "-c", command],
            "timeout": self.config.timeout.as_secs(),
        });

        let url = format!("{}/v2/containers", self.config.api_url);

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&sandbox_body)
            .send()
            .context("failed to create Modal container")?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("Modal API error ({}): {}", status, body);
        }

        // Parse container response to get exec output or logs URL.
        #[derive(serde::Deserialize)]
        struct ContainerResponse {
            id: Option<String>,
            stdout: Option<String>,
            stderr: Option<String>,
            status: Option<String>,
            logs: Option<String>,
        }

        let container: ContainerResponse =
            serde_json::from_str(&body).context("failed to parse Modal container response")?;

        // If the API returns logs inline, use them.
        if let Some(logs) = container.logs {
            return Ok(logs);
        }

        if let (Some(stdout), Some(stderr)) = (container.stdout, container.stderr) {
            let combined = if stderr.is_empty() {
                stdout
            } else {
                format!("{}\nstderr: {}", stdout, stderr)
            };
            return Ok(combined);
        }

        // If we got a container ID, try to fetch logs from it.
        if let Some(id) = container.id {
            let log_url = format!("{}/v2/containers/{}/logs", self.config.api_url, id);
            let log_response = self
                .client
                .get(&log_url)
                .bearer_auth(&self.config.api_key)
                .send()
                .context("failed to fetch Modal container logs")?;

            let log_body = log_response
                .text()
                .unwrap_or_else(|_| format!("Modal container {}: no logs available", id));
            return Ok(log_body);
        }

        Ok("Modal: command accepted, no output captured".to_string())
    }

    /// Validate Modal connectivity by running a simple command.
    pub fn validate(&self) -> Result<()> {
        let output = self.execute("echo 'praxis-modal-ok'", None)?;
        if output.contains("praxis-modal-ok") {
            Ok(())
        } else {
            anyhow::bail!("Modal validation failed: unexpected output: {}", output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_config_from_env_missing_key() {
        unsafe { std::env::remove_var("PRAXIS_MODAL_API_KEY") };
        let result = ModalConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn modal_config_validate_env_missing() {
        unsafe { std::env::remove_var("PRAXIS_MODAL_API_KEY") };
        let result = ModalConfig::validate_environment();
        assert!(result.is_err());
    }
}
