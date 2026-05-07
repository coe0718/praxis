//! Spotify integration — playback control via Spotify Web API.
//!
//! #26 Native Spotify
//!
//! Tools: play, pause, skip, search, queue, current_track, devices, transfer.
//!
//! Requires Spotify OAuth app with client ID/secret in config.
//! Set `SPOTIFY_CLIENT_ID` and `SPOTIFY_CLIENT_SECRET` env vars or config.
//!
//! Usage: `praxis spotify play "song name"` or `praxis spotify search --query "..."`
//!
//! API docs: https://developer.spotify.com/documentation/web-api

use std::collections::HashMap;
use std::env;
use std::fs;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Spotify API base URL.
const SPOTIFY_API: &str = "https://api.spotify.com/v1";

/// Spotify auth token endpoint.
const SPOTIFY_AUTH: &str = "https://accounts.spotify.com/api/token";

/// Config for Spotify client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            client_id: env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
            client_secret: env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: "http://localhost:8888/callback".to_string(),
            access_token: None,
            refresh_token: None,
        }
    }
}

impl SpotifyConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).context("read spotify config")?;
        serde_json::from_str(&raw).context("parse spotify config")
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("serialize spotify config")?;
        fs::write(path, json).context("write spotify config")
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }
}

/// Full Spotify client with auth and API calls.
pub struct SpotifyClient {
    config: SpotifyConfig,
    http: reqwest::blocking::Client,
}

impl SpotifyClient {
    pub fn new(config: SpotifyConfig) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("build reqwest blocking client")?;
        Ok(Self { config, http })
    }

    /// Exchange authorization code for tokens (PKCE OAuth flow).
    /// Call this after the user visits the auth URL from get_auth_url().
    pub fn exchange_code(&mut self, code: &str, code_verifier: &str) -> Result<()> {
        if !self.config.is_configured() {
            bail!("Spotify not configured — set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET");
        }

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &self.config.redirect_uri),
            ("client_id", &self.config.client_id),
            ("code_verifier", code_verifier),
        ];

        let resp = self
            .http
            .post(SPOTIFY_AUTH)
            .form(&params)
            .send()
            .context("Spotify token exchange")?;

        let status = resp.status();
        if !status.is_success() {
            bail!("Spotify auth failed ({}): {}", status, resp.text().unwrap_or_default());
        }

        let token: TokenResponse =
            serde_json::from_str(&resp.text().context("parse token response")?)?;
        self.config.access_token = Some(token.access_token);
        self.config.refresh_token = token.refresh_token;
        Ok(())
    }

    /// Refresh the access token using the refresh token.
    pub fn refresh(&mut self) -> Result<()> {
        let refresh = self.config.refresh_token.as_ref().context("no refresh token available")?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("client_id", &self.config.client_id),
        ];

        let resp = self
            .http
            .post(SPOTIFY_AUTH)
            .form(&params)
            .send()
            .context("Spotify token refresh")?;

        let token: TokenResponse =
            serde_json::from_str(&resp.text().context("parse refresh response")?)?;
        self.config.access_token = Some(token.access_token);
        if let Some(new_refresh) = token.refresh_token {
            self.config.refresh_token = Some(new_refresh);
        }
        Ok(())
    }

    /// Make an authenticated GET request to the Spotify API.
    fn get(&self, path: &str) -> Result<Value> {
        let token = self
            .config
            .access_token
            .as_ref()
            .context("not authenticated — run `praxis spotify auth` first")?;

        let resp = self
            .http
            .get(format!("{}{}", SPOTIFY_API, path))
            .bearer_auth(token)
            .send()
            .context("Spotify API request")?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            bail!("Spotify token expired — run `praxis spotify auth` to re-authenticate");
        }

        let status = resp.status();
        if !status.is_success() {
            bail!("Spotify API {} failed ({}): {}", path, status, resp.text().unwrap_or_default());
        }

        serde_json::from_str(&resp.text().context("parse Spotify response")?)
            .context("decode Spotify JSON")
    }

    /// Get the currently playing track.
    pub fn current_track(&self) -> Result<String> {
        let data: Value = self.get("/me/player/currently-playing")?;
        if data.is_null() {
            return Ok("Nothing playing".to_string());
        }
        let item = data.get("item").and_then(|i| i.as_object()).context("no track item")?;
        let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown");
        let artists = item
            .get("artists")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|a| {
                        a.as_object().and_then(|o| o.get("name").and_then(|n| n.as_str()))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Unknown".to_string());
        Ok(format!("{} — {}", name, artists))
    }

    /// Search Spotify.
    pub fn search(&self, query: &str, search_type: &str) -> Result<String> {
        let encoded = urlencoding::encode(query);
        let data: Value =
            self.get(&format!("/search?q={}&type={}&limit=10", encoded, search_type))?;

        let key = match search_type {
            "track" => "tracks",
            "artist" => "artists",
            "album" => "albums",
            "playlist" => "playlists",
            _ => "tracks",
        };

        let items = data
            .get(key)
            .and_then(|k| k.get("items"))
            .and_then(|i| i.as_array())
            .map(|arr| {
                arr.iter()
                    .take(5)
                    .map(|item| {
                        let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        let id = item.get("id").and_then(|i| i.as_str()).unwrap_or("");
                        format!("  - {} [{}]", name, id)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "  (no results)".to_string());

        Ok(format!("Search results for '{}':\n{}", query, items))
    }

    /// Pause playback.
    pub fn pause(&self) -> Result<String> {
        let token = self.config.access_token.as_ref().context("not authenticated")?;
        let resp = self
            .http
            .put(format!("{}/me/player/pause", SPOTIFY_API))
            .bearer_auth(token)
            .send()
            .context("Spotify pause")?;
        if resp.status().is_success() {
            Ok("Paused".to_string())
        } else {
            bail!("pause failed ({}): {}", resp.status(), resp.text().unwrap_or_default())
        }
    }

    /// Resume playback.
    pub fn play(&self, context_uri: Option<&str>, uris: Option<Vec<&str>>) -> Result<String> {
        let token = self.config.access_token.as_ref().context("not authenticated")?;

        let mut body: HashMap<&str, Value> = HashMap::new();
        if let Some(uri) = context_uri {
            body.insert("context_uri", serde_json::json!(uri));
        }
        if let Some(track_uris) = uris {
            body.insert("uris", serde_json::json!(track_uris));
        }

        let resp = self
            .http
            .put(format!("{}/me/player/play", SPOTIFY_API))
            .bearer_auth(token)
            .json(&body)
            .send()
            .context("Spotify play")?;

        if resp.status().is_success() {
            Ok("Playing".to_string())
        } else {
            bail!("play failed ({}): {}", resp.status(), resp.text().unwrap_or_default())
        }
    }

    /// Skip to next track.
    pub fn skip_next(&self) -> Result<String> {
        let token = self.config.access_token.as_ref().context("not authenticated")?;
        let resp = self
            .http
            .post(format!("{}/me/player/next", SPOTIFY_API))
            .bearer_auth(token)
            .send()
            .context("Spotify skip")?;
        if resp.status().is_success() {
            Ok("Skipped".to_string())
        } else {
            bail!("skip failed ({}): {}", resp.status(), resp.text().unwrap_or_default())
        }
    }

    /// Add a track to the user's queue.
    pub fn queue(&self, track_uri: &str) -> Result<String> {
        let token = self.config.access_token.as_ref().context("not authenticated")?;
        let resp = self
            .http
            .post(format!("{}/me/player/queue?uri={}", SPOTIFY_API, track_uri))
            .bearer_auth(token)
            .send()
            .context("Spotify queue")?;
        if resp.status().is_success() {
            Ok(format!("Queued: {}", track_uri))
        } else {
            bail!("queue failed ({}): {}", resp.status(), resp.text().unwrap_or_default())
        }
    }

    /// Get available devices.
    pub fn devices(&self) -> Result<String> {
        let data: Value = self.get("/me/player/devices")?;
        let devs = data
            .get("devices")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|d| {
                        let name = d.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        let is_active =
                            d.get("is_active").and_then(|a| a.as_bool()).unwrap_or(false);
                        let t = if is_active { " [ACTIVE]" } else { "" };
                        format!("  - {}{}", name, t)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| "  (no devices)".to_string());
        Ok(format!("Devices:\n{}", devs))
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    expires_in: u64,
}

/// Execute a Spotify tool command. Entry point from execute.rs.
/// Params: { "action": "search", "params": { "query": "...", "type": "track" } }
pub fn execute_spotify_tool(params: &serde_json::Value) -> Result<String> {
    let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("current_track");

    let config_file = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("praxis")
        .join("spotify.json");

    let config = SpotifyConfig::from_file(&config_file).unwrap_or_default();

    if !config.is_configured() {
        bail!(
            "Spotify not configured. Set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET env vars,\nor run `praxis spotify auth` to begin the OAuth flow.\n\
             Get your credentials at https://developer.spotify.com/dashboard"
        );
    }

    let mut client = SpotifyClient::new(config)?;

    // Try to refresh if token exists but might be expired
    if client.config.access_token.is_some() && client.config.refresh_token.is_some() {
        let _ = client.refresh();
    }

    let result = match action {
        "current_track" | "now_playing" => client.current_track(),
        "search" => {
            let query = params
                .get("params")
                .and_then(|p| p.get("query"))
                .and_then(|q| q.as_str())
                .unwrap_or("");
            let search_type = params
                .get("params")
                .and_then(|p| p.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("track");
            client.search(query, search_type)
        }
        "play" => {
            let context_uri =
                params.get("params").and_then(|p| p.get("context_uri")).and_then(|u| u.as_str());
            let uris: Option<Vec<&str>> = params
                .get("params")
                .and_then(|p| p.get("uris"))
                .and_then(|u| u.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());
            client.play(context_uri, uris)
        }
        "pause" => client.pause(),
        "skip" | "next" => client.skip_next(),
        "queue" => {
            let uri = params
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            client.queue(uri)
        }
        "devices" => client.devices(),
        other => bail!(
            "unknown spotify action: {}. Actions: current_track, search, play, pause, skip, queue, devices",
            other
        ),
    }?;

    // Save updated tokens
    if client.config.access_token.is_some() {
        let _ = client.config.save(&config_file);
    }

    Ok(result)
}
