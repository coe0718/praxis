use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

use super::OAuthTokenStore;

/// Minimal Gmail REST client using a stored Google OAuth token.
///
/// Scopes required: `https://www.googleapis.com/auth/gmail.readonly`
pub struct GmailClient {
    client: Client,
    access_token: String,
}

#[derive(Debug, Clone)]
pub struct EmailSummary {
    pub id: String,
    pub subject: String,
    pub from: String,
    pub snippet: String,
    pub date: String,
}

impl GmailClient {
    /// Load from the OAuth token store. Returns `None` when no Google token is stored.
    /// Automatically refreshes an expired or nearly-expired token if a refresh token is available.
    pub fn from_store(store: &OAuthTokenStore) -> Result<Option<Self>> {
        let mut token = match store.get("google")? {
            Some(t) => t,
            None => return Ok(None),
        };
        if token.needs_refresh() {
            match super::google::GoogleOAuth::from_env() {
                Ok(oauth) => {
                    match oauth.refresh(&token) {
                        Ok(new_token) => {
                            if let Err(e) = store.save(&new_token) {
                                log::warn!("failed to save refreshed google token: {e}");
                            }
                            token = new_token;
                        }
                        Err(e) => {
                            log::warn!("google token refresh failed: {e}");
                            if token.is_expired() {
                                anyhow::bail!(
                                    "Google OAuth token is expired and refresh failed — \
                                     run `praxis oauth login google` to re-authenticate"
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("google oauth env vars missing, cannot refresh: {e}");
                    if token.is_expired() {
                        anyhow::bail!(
                            "Google OAuth token is expired and cannot be refreshed — \
                             run `praxis oauth login google` to re-authenticate"
                        );
                    }
                }
            }
        }
        Ok(Some(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .context("failed to build HTTP client")?,
            access_token: token.access_token,
        }))
    }

    /// Return the `n` most recent messages from the inbox, ordered newest first.
    pub fn list_recent(&self, n: usize) -> Result<Vec<EmailSummary>> {
        let ids = self.list_message_ids(n)?;
        let mut summaries = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get_message_summary(&id) {
                Ok(s) => summaries.push(s),
                Err(e) => log::warn!("gmail: failed to fetch message {id}: {e}"),
            }
        }
        Ok(summaries)
    }

    fn list_message_ids(&self, n: usize) -> Result<Vec<String>> {
        let resp: ListMessagesResponse = self
            .client
            .get("https://gmail.googleapis.com/gmail/v1/users/me/messages")
            .bearer_auth(&self.access_token)
            .query(&[
                ("maxResults", n.to_string().as_str()),
                ("labelIds", "INBOX"),
                ("q", "is:unread"),
            ])
            .send()
            .context("failed to list Gmail messages")?
            .error_for_status()
            .context("Gmail list messages returned an error")?
            .json()
            .context("failed to parse Gmail list response")?;

        Ok(resp
            .messages
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.id)
            .collect())
    }

    fn get_message_summary(&self, id: &str) -> Result<EmailSummary> {
        let url = format!("https://gmail.googleapis.com/gmail/v1/users/me/messages/{id}");
        let msg: GmailMessage = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[
                ("format", "metadata"),
                ("metadataHeaders", "Subject,From,Date"),
            ])
            .send()
            .context("failed to GET Gmail message")?
            .error_for_status()
            .context("Gmail get message returned an error")?
            .json()
            .context("failed to parse Gmail message")?;

        let header = |name: &str| -> String {
            msg.payload
                .as_ref()
                .and_then(|p| p.headers.as_deref())
                .and_then(|hs| hs.iter().find(|h| h.name.eq_ignore_ascii_case(name)))
                .map(|h| h.value.clone())
                .unwrap_or_default()
        };

        Ok(EmailSummary {
            id: msg.id,
            subject: header("Subject"),
            from: header("From"),
            date: header("Date"),
            snippet: msg.snippet.unwrap_or_default(),
        })
    }
}

#[derive(Deserialize)]
struct ListMessagesResponse {
    messages: Option<Vec<MessageRef>>,
}

#[derive(Deserialize)]
struct MessageRef {
    id: String,
}

#[derive(Deserialize)]
struct GmailMessage {
    id: String,
    snippet: Option<String>,
    payload: Option<MessagePayload>,
}

#[derive(Deserialize)]
struct MessagePayload {
    headers: Option<Vec<MessageHeader>>,
}

#[derive(Deserialize)]
struct MessageHeader {
    name: String,
    value: String,
}
