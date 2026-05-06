//! Google Meet + Calendar integration via Google Calendar API.
//!
//! #27 Native Google Meet
//!
//! Tools: list_events, create_meeting, upcoming, join.
//!
//! Requires Google OAuth2 credentials. Set `GOOGLE_CLIENT_ID` and
//! `GOOGLE_CLIENT_SECRET` env vars or config file.
//!
//! API: https://developers.google.com/calendar/api/v3/reference
//!
//! Usage: `praxis meet list --days 7` or `praxis meet create "Meeting title"`

use std::env;
use std::fs;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Google Calendar API base.
const CAL_API: &str = "https://www.googleapis.com/calendar/v3";

/// Google OAuth token endpoint.
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Google OAuth authorize URL.
#[allow(dead_code)]
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Config for Google Meet client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expiry: Option<u64>,
}

impl Default for MeetConfig {
    fn default() -> Self {
        Self {
            client_id: env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            client_secret: env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: "http://localhost:8889/callback".to_string(),
            access_token: None,
            refresh_token: None,
            expiry: None,
        }
    }
}

impl MeetConfig {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).context("read meet config")?;
        serde_json::from_str(&raw).context("parse meet config")
    }

    pub fn save(&self, path: &std::path::Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("serialize meet config")?;
        fs::write(path, json).context("write meet config")
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    pub fn is_token_expired(&self) -> bool {
        if let Some(expiry) = self.expiry {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            expiry <= now + 60 // refresh 60s before expiry
        } else {
            true
        }
    }
}

/// Google Meet/Calendar client.
pub struct MeetClient {
    config: MeetConfig,
    http: reqwest::blocking::Client,
}

impl MeetClient {
    pub fn new(config: MeetConfig) -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("build reqwest client")?;
        Ok(Self { config, http })
    }

    /// Get an access token, refreshing if expired.
    fn get_token(&mut self) -> Result<&str> {
        if self.config.access_token.is_none() || self.config.is_token_expired() {
            self.refresh()?;
        }
        Ok(self.config.access_token.as_deref().unwrap())
    }

    /// Refresh access token.
    fn refresh(&mut self) -> Result<()> {
        let refresh = self
            .config
            .refresh_token
            .as_ref()
            .context("no refresh token — run `praxis meet auth` first")?;

        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let resp = self
            .http
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .context("Google token refresh")?;

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: u64,
            #[serde(default)]
            refresh_token: Option<String>,
        }

        let token: TokenResp = serde_json::from_str(&resp.text().context("parse token response")?)
            .context("decode token JSON")?;

        self.config.access_token = Some(token.access_token);
        if let Some(rt) = token.refresh_token {
            self.config.refresh_token = Some(rt);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.config.expiry = Some(now + token.expires_in);
        Ok(())
    }

    /// Make an authenticated GET request to Google Calendar API.
    fn get(&mut self, path: &str) -> Result<serde_json::Value> {
        let token = self.get_token()?.to_string();
        let resp = self
            .http
            .get(format!("{}{}", CAL_API, path))
            .bearer_auth(&token)
            .send()
            .context("Google Calendar API request")?;

        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            bail!("Google token expired — run `praxis meet auth` to re-authenticate");
        }

        let status = resp.status();
        if !status.is_success() {
            bail!(
                "Google Calendar API {} failed ({}): {}",
                path,
                status,
                resp.text().unwrap_or_default()
            );
        }

        serde_json::from_str(&resp.text().context("parse Google response")?)
            .context("decode Google JSON")
    }

    /// List upcoming calendar events.
    pub fn list_events(&mut self, days: u32) -> Result<String> {
        let now = chrono_lite_rfc3339();
        let max_time = plus_days(days);

        let data = self.get(&format!(
            "/calendars/primary/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=20",
            now, max_time
        ))?;

        let items = data
            .get("items")
            .and_then(|i| i.as_array())
            .map(|arr| {
                if arr.is_empty() {
                    return String::from("  (no upcoming events)");
                }
                arr.iter()
                    .filter_map(|item| {
                        let summary =
                            item.get("summary").and_then(|s| s.as_str()).unwrap_or("(no title)");
                        let start = item
                            .get("start")
                            .and_then(|s| s.get("dateTime").or(s.get("date")))
                            .and_then(|d| d.as_str())
                            .unwrap_or("?");
                        let link = item.get("htmlLink").and_then(|l| l.as_str()).unwrap_or("");
                        let meet_link = item
                            .get("conferenceData")
                            .and_then(|cd| cd.get("entryPoints"))
                            .and_then(|ep| ep.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|e| e.get("uri"))
                            .and_then(|u| u.as_str())
                            .unwrap_or("");
                        let location = if !meet_link.is_empty() {
                            format!(" 🔗 {}", meet_link)
                        } else {
                            String::new()
                        };
                        Some(format!("  • {} — {}{}  ({})", summary, start, location, link))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_else(|| String::from("  (could not parse events)"));

        Ok(format!("Upcoming events (next {} days):\n{}", days, items))
    }

    /// Create a calendar event with Google Meet link.
    pub fn create_meeting(&mut self, title: &str, duration_mins: u32) -> Result<String> {
        let token = self.get_token()?.to_string();
        let start = chrono_rfc3339_offset(duration_mins as i64);
        let end = chrono_rfc3339_offset(0);

        let body = serde_json::json!({
            "summary": title,
            "start": { "dateTime": start, "timeZone": "UTC" },
            "end": { "dateTime": end, "timeZone": "UTC" },
            "conferenceData": {
                "createRequest": {
                    "conferenceSolutionKey": { "type": "hangoutsMeet" },
                    "requestId": format!("praxis-{}", std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos())
                }
            },
            "reminders": {
                "useDefault": false,
                "overrides": [
                    { "method": "popup", "minutes": 10 }
                ]
            }
        });

        let resp = self
            .http
            .post(format!("{}/calendars/primary/events?conferenceDataVersion=1", CAL_API))
            .bearer_auth(token)
            .json(&body)
            .send()
            .context("create calendar event")?;

        let status = resp.status();
        let text = resp.text().context("read create response")?;

        if status.as_u16() == 403 {
            bail!(
                "Calendar API permission denied — ensure Google Calendar API is enabled and scopes include calendar.events."
            );
        }

        if !status.is_success() {
            bail!("create meeting failed ({}): {}", status, text);
        }

        let data: serde_json::Value =
            serde_json::from_str(&text).context("parse create response")?;

        let meet_link = data
            .get("conferenceData")
            .and_then(|cd| cd.get("entryPoints"))
            .and_then(|ep| ep.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("uri"))
            .and_then(|u| u.as_str())
            .unwrap_or("https://meet.google.com");

        let event_link = data.get("htmlLink").and_then(|l| l.as_str()).unwrap_or("");

        Ok(format!(
            "Meeting created: {}\nJoin: {}\nCalendar: {}",
            title, meet_link, event_link
        ))
    }
}

/// Returns current RFC3339 timestamp.
fn chrono_lite_rfc3339() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs() as i64;
    // Simple RFC3339 without chrono dep
    let (y, m, d, hh, mm, ss) = civil_from_epoch(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Returns RFC3339 timestamp +N days.
fn plus_days(days: u32) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs() as i64 + (days as i64 * 86400);
    let (y, m, d, hh, mm, ss) = civil_from_epoch(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Returns RFC3339 timestamp starting at now + offset minutes.
fn chrono_rfc3339_offset(offset_mins: i64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs() as i64 + (offset_mins * 60);
    let (y, m, d, hh, mm, ss) = civil_from_epoch(secs);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Convert Unix timestamp to (year, month, day, hour, min, sec) UTC.
fn civil_from_epoch(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let mut days = secs / 86400;
    let mut secs_in_day = secs % 86400;
    if secs_in_day < 0 {
        days -= 1;
        secs_in_day += 86400;
    }

    // 1970-01-01 is day 0. Calculate year.
    let mut year = 1970;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    static DAYS_IN_MONTH: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    let days_in_month = if is_leap(year) { 29 } else { 28 };
    let mut d = days as u32 + 1;
    for &dim in &DAYS_IN_MONTH {
        if month == 2 && is_leap(year) {
            if d > days_in_month {
                d -= days_in_month;
                month += 1;
                continue;
            }
            break;
        }
        if d > dim {
            d -= dim;
            month += 1;
        } else {
            break;
        }
    }

    let hour = (secs_in_day / 3600) as u32;
    let min = ((secs_in_day % 3600) / 60) as u32;
    let second = (secs_in_day % 60) as u32;
    (year, month, d, hour, min, second)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Execute a Google Meet tool command.
/// Params: { "action": "list_events", "params": { "days": 7 } }
pub fn execute_meet_tool(params: &serde_json::Value) -> Result<String> {
    let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("list_events");

    let config_file = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("praxis")
        .join("meet.json");

    let config = MeetConfig::from_file(&config_file).unwrap_or_default();

    if !config.is_configured() {
        bail!(
            "Google Meet not configured. Set GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET env vars,\nor run `praxis meet auth` to begin OAuth flow.\n\
             Enable Google Calendar API at https://console.cloud.google.com/apis/library/calendar-json.googleapis.com"
        );
    }

    let mut client = MeetClient::new(config)?;

    let result = match action {
        "list_events" | "upcoming" => {
            let days = params
                .get("params")
                .and_then(|p| p.get("days"))
                .and_then(|d| d.as_u64())
                .unwrap_or(7) as u32;
            client.list_events(days)
        }
        "create" | "create_meeting" => {
            let title = params
                .get("params")
                .and_then(|p| p.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("Praxis Meeting");
            let duration = params
                .get("params")
                .and_then(|p| p.get("duration_mins"))
                .and_then(|d| d.as_u64())
                .unwrap_or(60) as u32;
            client.create_meeting(title, duration)
        }
        other => bail!("unknown meet action: {}. Actions: list_events, create", other),
    }?;

    // Save updated token
    if client.config.access_token.is_some() {
        let _ = client.config.save(&config_file);
    }

    Ok(result)
}
