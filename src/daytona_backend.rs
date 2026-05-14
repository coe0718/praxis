//! Daytona Terminal Backend — Execute tools in Daytona container sandboxes.
//!
//! Daytona provides managed, per-project development environments via a REST API.
//! Docs: <https://docs.daytona.com>
//!
//! Set `PRAXIS_DAYTONA_API_KEY`, `PRAXIS_DAYTONA_API_URL`, and optionally
//! `PRAXIS_DAYTONA_DEFAULT_IMAGE`.
use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use std::time::Duration;

/// Daytona API configuration.
#[derive(Debug, Clone)]
pub struct DaytonaConfig {
    api_key: String,
    api_url: String,
    default_image: String,
    timeout: Duration,
}

impl DaytonaConfig {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("PRAXIS_DAYTONA_API_KEY")
            .context("PRAXIS_DAYTONA_API_KEY is required for Daytona backend")?;
        let api_url = std::env::var("PRAXIS_DAYTONA_API_URL")
            .unwrap_or_else(|_| "https://api.daytona.com".to_string());
        let default_image = std::env::var("PRAXIS_DAYTONA_DEFAULT_IMAGE")
            .unwrap_or_else(|_| "daytonaio/ubuntu:latest".to_string());
        let timeout_secs: u64 = std::env::var("PRAXIS_DAYTONA_TIMEOUT")
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
        if std::env::var("PRAXIS_DAYTONA_API_KEY").is_err() {
            anyhow::bail!("PRAXIS_DAYTONA_API_KEY is required for Daytona backend");
        }
        Ok(())
    }
}

/// A Daytona project sandbox for executing commands.
pub struct DaytonaBackend {
    config: DaytonaConfig,
    client: Client,
}

impl DaytonaBackend {
    pub fn new(config: DaytonaConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { config, client }
    }

    /// Execute a command inside a Daytona project environment.
    ///
    /// Uses the Daytona Project API to run commands inside isolated dev containers.
    pub fn execute(&self, command: &str, image: Option<&str>) -> Result<String> {
        let image = image.unwrap_or(&self.config.default_image);

        // Step 1: Create a new Daytona project.
        let create_body = serde_json::json!({
            "image": image,
            "ide": "none",
            "name": format!("praxis-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("x")),
            "target": "default",
            "repo": "",
        });

        let url = format!("{}/project", self.config.api_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&create_body)
            .send()
            .context("failed to create Daytona project")?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!("Daytona API error ({}): {}", status, body);
        }

        #[derive(serde::Deserialize)]
        struct ProjectResponse {
            repo: Option<RepoInfo>,
            name: Option<String>,
            id: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct RepoInfo {
            branch: Option<String>,
            #[serde(rename = "sha")]
            commit: Option<String>,
            owner: Option<String>,
            repo: Option<String>,
        }

        let project: ProjectResponse =
            serde_json::from_str(&body).context("failed to parse Daytona project response")?;

        let project_id = project.id.context("Daytona response missing project ID")?;
        if let Some(ref name) = project.name {
            log::info!("Daytona: created project '{name}' ({project_id})");
        }
        if let Some(ref repo) = project.repo {
            log::info!(
                "Daytona: repo {}/{} @ {} ({})",
                repo.owner.as_deref().unwrap_or("?"),
                repo.repo.as_deref().unwrap_or("?"),
                repo.branch.as_deref().unwrap_or("HEAD"),
                repo.commit.as_deref().unwrap_or("?")
            );
        }

        // Step 2: Execute a command in the running project.
        let exec_body = serde_json::json!({
            "command": ["/bin/sh", "-c", command],
            "projectId": project_id,
        });

        let exec_url = format!("{}/project/{}", self.config.api_url, project_id);

        let exec_response = self
            .client
            .patch(&exec_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&exec_body)
            .send()
            .context("failed to execute command in Daytona project")?;

        let exec_status = exec_response.status();
        let exec_body_text = exec_response.text().unwrap_or_default();

        if !exec_status.is_success() {
            bail!("Daytona exec error ({}): {}", exec_status, exec_body_text);
        }

        Ok(exec_body_text)
    }

    /// Validate Daytona connectivity by running a simple command.
    pub fn validate(&self) -> Result<()> {
        let output = self.execute("echo 'praxis-daytona-ok'", None)?;
        if output.contains("praxis-daytona-ok") {
            Ok(())
        } else {
            anyhow::bail!("Daytona validation failed: unexpected output: {}", output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daytona_config_from_env_missing_key() {
        unsafe { std::env::remove_var("PRAXIS_DAYTONA_API_KEY") };
        let result = DaytonaConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn daytona_config_validate_env_missing() {
        unsafe { std::env::remove_var("PRAXIS_DAYTONA_API_KEY") };
        let result = DaytonaConfig::validate_environment();
        assert!(result.is_err());
    }
}
