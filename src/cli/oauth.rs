use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::{
    oauth::{
        GitHubOAuth, GoogleOAuth, OAuthTokenStore,
        github::DEFAULT_SCOPES as GITHUB_DEFAULT_SCOPES,
        google::DEFAULT_SCOPES as GOOGLE_DEFAULT_SCOPES,
    },
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct OAuthArgs {
    #[command(subcommand)]
    command: OAuthCommand,
}

#[derive(Debug, Subcommand)]
enum OAuthCommand {
    /// Authorize Praxis to access a service via OAuth device flow.
    Login(OAuthLoginArgs),
    /// Show authorization status for all providers.
    Status,
    /// Print the raw access token for a provider (for use in shell tools).
    Token(OAuthProviderArgs),
    /// Force a token refresh (Google only — GitHub tokens do not expire).
    Refresh(OAuthProviderArgs),
    /// Remove stored authorization for a provider.
    Revoke(OAuthProviderArgs),
}

#[derive(Debug, Args)]
struct OAuthLoginArgs {
    /// Provider to authorize (github, google).
    provider: String,

    /// Override the default OAuth scopes (space or comma-separated).
    #[arg(long)]
    scopes: Option<String>,
}

#[derive(Debug, Args)]
struct OAuthProviderArgs {
    /// Provider name (github, google).
    provider: String,
}

pub fn handle_oauth(data_dir_override: Option<PathBuf>, args: OAuthArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = OAuthTokenStore::new(&paths.data_dir);

    match args.command {
        OAuthCommand::Login(a) => handle_login(&store, a),
        OAuthCommand::Status => handle_status(&store),
        OAuthCommand::Token(a) => handle_token(&store, &a.provider),
        OAuthCommand::Refresh(a) => handle_refresh(&store, &a.provider),
        OAuthCommand::Revoke(a) => handle_revoke(&store, &a.provider),
    }
}

fn handle_login(store: &OAuthTokenStore, args: OAuthLoginArgs) -> Result<String> {
    match args.provider.to_lowercase().as_str() {
        "github" => {
            let scopes = args.scopes.as_deref().unwrap_or(GITHUB_DEFAULT_SCOPES);
            let client = GitHubOAuth::from_env()?;
            let token = client.login(scopes)?;
            let scopes_str = token.scopes.join(", ");
            store.save(&token)?;
            Ok(format!("github: authorized\nscopes: {scopes_str}"))
        }
        "google" => {
            let scopes = args.scopes.as_deref().unwrap_or(GOOGLE_DEFAULT_SCOPES);
            let client = GoogleOAuth::from_env()?;
            let token = client.login(scopes)?;
            let scopes_str = token.scopes.join(", ");
            let expires = token
                .expires_at
                .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "never".to_string());
            store.save(&token)?;
            Ok(format!(
                "google: authorized\nscopes: {scopes_str}\nexpires: {expires}"
            ))
        }
        other => bail!("unknown provider '{other}' — supported: github, google"),
    }
}

fn handle_status(store: &OAuthTokenStore) -> Result<String> {
    let tokens = store.load()?;

    if tokens.is_empty() {
        return Ok(
            "No OAuth providers authorized.\n\
             Run `praxis oauth login github` or `praxis oauth login google` to connect."
                .to_string(),
        );
    }

    let mut lines = Vec::new();
    for provider in ["github", "google"] {
        match tokens.get(provider) {
            Some(token) => {
                let status = if token.is_expired() {
                    "EXPIRED".to_string()
                } else if token.needs_refresh() {
                    "expires soon".to_string()
                } else {
                    "ok".to_string()
                };
                let expiry = token
                    .expires_at
                    .map(|t| format!(" (expires {})", t.format("%Y-%m-%d %H:%M UTC")))
                    .unwrap_or_default();
                let since = token.authorized_at.format("%Y-%m-%d").to_string();
                let scopes = token.scopes.join(", ");
                lines.push(format!(
                    "{provider}: {status}{expiry}\n  authorized: {since}\n  scopes: {scopes}"
                ));
            }
            None => {
                lines.push(format!("{provider}: not authorized"));
            }
        }
    }
    Ok(lines.join("\n"))
}

fn handle_token(store: &OAuthTokenStore, provider: &str) -> Result<String> {
    let prov = provider.to_lowercase();
    let token = store
        .get(&prov)?
        .with_context(|| format!("no token for '{provider}' — run `praxis oauth login {provider}`"))?;

    if token.is_expired() {
        bail!(
            "token for '{provider}' has expired — run `praxis oauth refresh {provider}` or re-login"
        );
    }

    Ok(token.access_token)
}

fn handle_refresh(store: &OAuthTokenStore, provider: &str) -> Result<String> {
    let prov = provider.to_lowercase();
    match prov.as_str() {
        "google" => {
            let token = store.get("google")?.with_context(|| {
                "no Google token stored — run `praxis oauth login google` first".to_string()
            })?;
            let client = GoogleOAuth::from_env()?;
            let refreshed = client.refresh(&token)?;
            let expires = refreshed
                .expires_at
                .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "never".to_string());
            store.save(&refreshed)?;
            Ok(format!("google: token refreshed\nexpires: {expires}"))
        }
        "github" => Ok(
            "github: tokens do not expire by default — no refresh needed.\n\
             If your token was revoked, run `praxis oauth login github` to re-authorize."
                .to_string(),
        ),
        other => bail!("unknown provider '{other}' — supported: github, google"),
    }
}

fn handle_revoke(store: &OAuthTokenStore, provider: &str) -> Result<String> {
    let prov = provider.to_lowercase();
    let removed = store.remove(&prov)?;
    if removed {
        Ok(format!("{provider}: authorization removed"))
    } else {
        Ok(format!("{provider}: was not authorized"))
    }
}
