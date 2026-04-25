//! GitHub Copilot OAuth — device flow for LLM access.
//!
//! Copilot reuses GitHub's OAuth infrastructure.  The token is obtained via
//! the same device-flow endpoint as `github.rs`, but with the `copilot`
//! scope (plus `read:org` for seat verification).
//!
//! The access token is then used as a Bearer token against the Copilot
//! Chat Completions endpoint, which is OpenAI-compatible.

use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::blocking::Client;

use super::{
    device_flow::{DeviceFlowConfig, poll_for_token, request_device_code},
    store::OAuthToken,
};

const CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Scopes required for Copilot Chat API access.
pub const DEFAULT_SCOPES: &str = "read:org copilot";

pub struct CopilotOAuth {
    client_id: String,
    client: Client,
}

impl CopilotOAuth {
    /// Load credentials from `PRAXIS_COPILOT_OAUTH_CLIENT_ID`.
    ///
    /// Register a new OAuth App at https://github.com/settings/developers
    /// and set the callback URL to `http://localhost` (device flow does not
    /// actually redirect).
    pub fn from_env() -> Result<Self> {
        let client_id = std::env::var("PRAXIS_COPILOT_OAUTH_CLIENT_ID").context(
            "PRAXIS_COPILOT_OAUTH_CLIENT_ID is required for Copilot OAuth.
Register an OAuth App at https://github.com/settings/developers",
        )?;
        Ok(Self {
            client_id,
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("failed to build HTTP client")?,
        })
    }

    /// Run the device authorization flow interactively.
    pub fn login(&self, scopes: &str) -> Result<OAuthToken> {
        let config = DeviceFlowConfig {
            code_url: CODE_URL,
            token_url: TOKEN_URL,
            client_id: &self.client_id,
            client_secret: None,
            scope: scopes,
            grant_type: GRANT_TYPE,
        };

        let device = request_device_code(&self.client, &config)?;

        println!(
            "\nGitHub Copilot authorization required.\n  Open: {}\n  Enter code: {}\n\nWaiting for authorization...",
            device.verification_uri, device.user_code
        );

        let result = poll_for_token(
            &self.client,
            &config,
            &device.device_code,
            device.interval,
            device.expires_in,
        )?;

        // GitHub tokens from device flow do not expire by default, but we
        // record expires_in if the provider ever starts returning it.
        let expires_at = result
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64));

        let scope_str = result.scope.as_deref().unwrap_or(scopes);
        let scopes_list = scope_str.split(|c| c == ' ' || c == ',').map(str::to_string).collect();

        Ok(OAuthToken {
            provider: "copilot".to_string(),
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            scopes: scopes_list,
            expires_at,
            authorized_at: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_scopes_include_copilot() {
        assert!(DEFAULT_SCOPES.contains("copilot"));
        assert!(DEFAULT_SCOPES.contains("read:org"));
    }
}
