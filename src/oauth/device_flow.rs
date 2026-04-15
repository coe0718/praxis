use std::{thread, time::Duration};

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::Deserialize;

pub struct DeviceFlowConfig<'a> {
    pub code_url: &'a str,
    pub token_url: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub scope: &'a str,
    pub grant_type: &'a str,
}

pub struct DeviceFlowResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
    pub token_type: String,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    /// Google calls this `verification_url`; RFC 8628 calls it `verification_uri`.
    #[serde(alias = "verification_url")]
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct TokenPollResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Step 1: request a device code and user-facing code from the provider.
pub fn request_device_code(
    client: &Client,
    config: &DeviceFlowConfig<'_>,
) -> Result<DeviceCodeResponse> {
    let params = vec![("client_id", config.client_id), ("scope", config.scope)];

    let response = client
        .post(config.code_url)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .context("failed to request device code")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("device code request failed with {status}: {body}");
    }

    response
        .json::<DeviceCodeResponse>()
        .context("failed to parse device code response")
}

/// Step 2: poll the token endpoint until the user completes authorization.
pub fn poll_for_token(
    client: &Client,
    config: &DeviceFlowConfig<'_>,
    device_code: &str,
    interval_secs: u64,
    expires_in: u64,
) -> Result<DeviceFlowResult> {
    let deadline = std::time::Instant::now() + Duration::from_secs(expires_in);
    let mut poll_secs = interval_secs.max(5);

    loop {
        if std::time::Instant::now() >= deadline {
            bail!("device authorization expired before the user completed login");
        }

        thread::sleep(Duration::from_secs(poll_secs));

        let mut form: Vec<(String, String)> = vec![
            ("client_id".into(), config.client_id.into()),
            ("device_code".into(), device_code.into()),
            ("grant_type".into(), config.grant_type.into()),
        ];
        if let Some(secret) = config.client_secret {
            form.push(("client_secret".into(), secret.into()));
        }

        let response = client
            .post(config.token_url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .context("failed to poll token endpoint")?;

        let parsed: TokenPollResponse = response
            .json()
            .context("failed to parse token poll response")?;

        match parsed.error.as_deref() {
            None => {
                let access_token = parsed
                    .access_token
                    .filter(|t| !t.is_empty())
                    .context("token response missing access_token")?;
                return Ok(DeviceFlowResult {
                    access_token,
                    refresh_token: parsed.refresh_token,
                    expires_in: parsed.expires_in,
                    token_type: parsed.token_type.unwrap_or_else(|| "bearer".to_string()),
                    scope: parsed.scope,
                });
            }
            Some("authorization_pending") => {
                // User hasn't acted yet — keep polling at the same rate.
            }
            Some("slow_down") => {
                // Provider asked us to back off.
                poll_secs += 5;
            }
            Some("access_denied") => {
                bail!("the user denied the authorization request");
            }
            Some("expired_token") => {
                bail!("device code expired — please run `praxis oauth login` again");
            }
            Some(other) => {
                bail!("authorization failed: {other}");
            }
        }
    }
}
