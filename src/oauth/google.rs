use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;

use super::{
    device_flow::{DeviceFlowConfig, poll_for_token, request_device_code},
    store::OAuthToken,
};

const CODE_URL: &str = "https://accounts.google.com/o/oauth2/device/code";
const TOKEN_URL: &str = "https://accounts.google.com/o/oauth2/token";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

/// Default scopes for a personal agent:
/// - Gmail read — morning brief can summarise emails.
/// - Calendar full — scheduling and event awareness.
/// - Drive read — access documents and files.
/// - userinfo — display name / email for status output.
pub const DEFAULT_SCOPES: &str = concat!(
    "https://www.googleapis.com/auth/gmail.readonly ",
    "https://www.googleapis.com/auth/calendar ",
    "https://www.googleapis.com/auth/drive.readonly ",
    "https://www.googleapis.com/auth/userinfo.email ",
    "https://www.googleapis.com/auth/userinfo.profile",
);

pub struct GoogleOAuth {
    client_id: String,
    client_secret: String,
    client: Client,
}

impl GoogleOAuth {
    /// Load credentials from environment variables.
    ///
    /// Set up a project in Google Cloud Console, create an OAuth 2.0 client of
    /// type "TV and limited input devices", and set:
    /// - `PRAXIS_GOOGLE_OAUTH_CLIENT_ID`
    /// - `PRAXIS_GOOGLE_OAUTH_CLIENT_SECRET`
    pub fn from_env() -> Result<Self> {
        let client_id = std::env::var("PRAXIS_GOOGLE_OAUTH_CLIENT_ID").context(
            "PRAXIS_GOOGLE_OAUTH_CLIENT_ID is required for Google OAuth.\n\
             Create a 'TV and limited input devices' OAuth client at \
             https://console.cloud.google.com/apis/credentials",
        )?;
        let client_secret = std::env::var("PRAXIS_GOOGLE_OAUTH_CLIENT_SECRET")
            .context("PRAXIS_GOOGLE_OAUTH_CLIENT_SECRET is required for Google OAuth.")?;
        Ok(Self {
            client_id,
            client_secret,
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
            client_secret: Some(&self.client_secret),
            scope: scopes,
            grant_type: GRANT_TYPE,
        };

        let device = request_device_code(&self.client, &config)?;

        println!(
            "\nGoogle authorization required.\n  Open: {}\n  Enter code: {}\n\nWaiting for authorization...",
            device.verification_uri, device.user_code
        );

        let result = poll_for_token(
            &self.client,
            &config,
            &device.device_code,
            device.interval,
            device.expires_in,
        )?;

        let expires_at = result
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64));

        // Google returns scopes space-separated.
        let scope_str = result.scope.as_deref().unwrap_or(scopes);
        let scopes_list = scope_str.split_whitespace().map(str::to_string).collect();

        Ok(OAuthToken {
            provider: "google".to_string(),
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            scopes: scopes_list,
            expires_at,
            authorized_at: Utc::now(),
        })
    }

    /// Exchange a refresh token for a fresh access token.
    pub fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken> {
        let refresh_token = token.refresh_token.as_deref().context(
            "google token has no refresh_token — please run `praxis oauth login google` again",
        )?;

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];

        let response = self
            .client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .context("failed to call Google token refresh endpoint")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Google token refresh failed with {status}: {body}");
        }

        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            expires_in: Option<u64>,
            token_type: Option<String>,
        }

        let parsed: RefreshResponse =
            response.json().context("failed to parse Google token refresh response")?;

        let expires_at: Option<DateTime<Utc>> = parsed
            .expires_in
            .map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64));

        Ok(OAuthToken {
            provider: "google".to_string(),
            access_token: parsed.access_token,
            refresh_token: token.refresh_token.clone(), // carry the existing refresh token forward
            token_type: parsed.token_type.unwrap_or_else(|| "bearer".to_string()),
            scopes: token.scopes.clone(),
            expires_at,
            authorized_at: token.authorized_at,
        })
    }
}
