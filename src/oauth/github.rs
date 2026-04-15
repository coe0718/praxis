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

/// Default scopes: full repository access, user profile, org membership.
pub const DEFAULT_SCOPES: &str = "repo user read:org";

pub struct GitHubOAuth {
    client_id: String,
    client: Client,
}

impl GitHubOAuth {
    /// Load the client ID from `PRAXIS_GITHUB_OAUTH_CLIENT_ID`.
    pub fn from_env() -> Result<Self> {
        let client_id = std::env::var("PRAXIS_GITHUB_OAUTH_CLIENT_ID").context(
            "PRAXIS_GITHUB_OAUTH_CLIENT_ID is required for GitHub OAuth.\n\
             Create an OAuth App at https://github.com/settings/developers and \
             set the env var to its Client ID.",
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
    ///
    /// Prints the user code and verification URL to stdout, then polls until
    /// the user completes authorization in their browser.
    pub fn login(&self, scopes: &str) -> Result<OAuthToken> {
        let config = DeviceFlowConfig {
            code_url: CODE_URL,
            token_url: TOKEN_URL,
            client_id: &self.client_id,
            client_secret: None, // GitHub device flow does not need a secret.
            scope: scopes,
            grant_type: GRANT_TYPE,
        };

        let device = request_device_code(&self.client, &config)?;

        println!(
            "\nGitHub authorization required.\n  Open: {}\n  Enter code: {}\n\nWaiting for authorization...",
            device.verification_uri, device.user_code
        );

        let result = poll_for_token(
            &self.client,
            &config,
            &device.device_code,
            device.interval,
            device.expires_in,
        )?;

        // GitHub returns scopes as a comma-separated string.
        let scope_str = result.scope.as_deref().unwrap_or(scopes);
        let scopes_list = scope_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(OAuthToken {
            provider: "github".to_string(),
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            scopes: scopes_list,
            expires_at: None, // GitHub tokens do not expire by default.
            authorized_at: Utc::now(),
        })
    }
}
