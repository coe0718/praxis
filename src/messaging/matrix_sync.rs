//! Matrix real-time sync — WebSocket-based inbound message handling.
//!
//! Connects to a Matrix homeserver via WebSocket (or falls back to long-polling)
//! to receive messages in real-time and publish them to the Praxis message bus.
//!
//! Requires: `PRAXIS_MATRIX_HOMESERVER`, `PRAXIS_MATRIX_ACCESS_TOKEN`, `PRAXIS_MATRIX_USER_ID`
//! Optional: `PRAXIS_MATRIX_SYNC_FILTER` (JSON filter for event types)
use crate::bus::MessageBus;
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Matrix client configuration.
#[derive(Debug, Clone)]
pub struct MatrixSyncConfig {
    pub homeserver: String,
    pub access_token: String,
    pub user_id: String,
    pub filter: Option<String>,
    pub sync_timeout_secs: u64,
}

impl MatrixSyncConfig {
    pub fn from_env() -> Result<Self> {
        let homeserver = std::env::var("PRAXIS_MATRIX_HOMESERVER")
            .context("PRAXIS_MATRIX_HOMESERVER is required for Matrix sync")?;
        let access_token = std::env::var("PRAXIS_MATRIX_ACCESS_TOKEN")
            .context("PRAXIS_MATRIX_ACCESS_TOKEN is required for Matrix sync")?;
        let user_id = std::env::var("PRAXIS_MATRIX_USER_ID")
            .context("PRAXIS_MATRIX_USER_ID is required for Matrix sync")?;
        let filter = std::env::var("PRAXIS_MATRIX_SYNC_FILTER").ok();
        let sync_timeout_secs: u64 = std::env::var("PRAXIS_MATRIX_SYNC_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30_000);

        Ok(Self {
            homeserver: homeserver.trim_end_matches('/').to_string(),
            access_token,
            user_id,
            filter,
            sync_timeout_secs,
        })
    }

    pub fn validate_environment() -> Result<()> {
        let missing = [
            ("PRAXIS_MATRIX_HOMESERVER", std::env::var("PRAXIS_MATRIX_HOMESERVER")),
            ("PRAXIS_MATRIX_ACCESS_TOKEN", std::env::var("PRAXIS_MATRIX_ACCESS_TOKEN")),
            ("PRAXIS_MATRIX_USER_ID", std::env::var("PRAXIS_MATRIX_USER_ID")),
        ]
        .into_iter()
        .filter_map(|(name, v)| v.err().map(|_| name))
        .collect::<Vec<_>>();

        if !missing.is_empty() {
            anyhow::bail!("Missing Matrix environment variables: {}", missing.join(", "));
        }
        Ok(())
    }
}

/// Represents a Matrix room the bot should sync with.
#[derive(Debug, Clone)]
pub struct MatrixRoom {
    pub room_id: String,
    pub display_name: Option<String>,
}

/// Matrix sync session — manages the sync loop and message dispatch.
pub struct MatrixSync {
    config: MatrixSyncConfig,
    http_client: Client,
    next_batch: Option<String>,
}

impl MatrixSync {
    pub fn new(config: MatrixSyncConfig) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.sync_timeout_secs))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            config,
            http_client,
            next_batch: None,
        }
    }

    /// Run the Matrix sync loop indefinitely, using WebSocket when available
    /// and falling back to long-polling.
    pub async fn run_sync(&mut self, bus: &crate::bus::FileBus) -> Result<()> {
        // Try WebSocket first, fall back to long-polling.
        if self.try_websocket_sync(bus).await.is_err() {
            log::warn!("matrix: WebSocket sync failed, falling back to long-polling");
            self.long_poll_sync(bus).await?;
        }
        Ok(())
    }

    /// Attempt WebSocket-based real-time sync.
    async fn try_websocket_sync(&mut self, bus: &crate::bus::FileBus) -> Result<()> {
        // First, make an initial sync to get the filter and obtain a sync token.
        let filter = self.config.filter.clone().unwrap_or_else(|| {
            json!({
                "room": {
                    "timeline": { "limit": 0 },
                    "state": { "lazy_load_members": true }
                },
                "account_data": { "limit": 0 },
                "presence": { "limit": 0 }
            })
            .to_string()
        });

        let ws_url = format!(
            "{}/_matrix/client/v3/ws?access_token={}",
            self.config.homeserver, self.config.access_token
        );

        log::info!("matrix: connecting to WebSocket at {}", ws_url);

        let (ws_stream, _) =
            connect_async(&ws_url).await.context("failed to connect to Matrix WebSocket")?;

        let (mut write, mut read) = ws_stream.split();

        // Send the initial sync filter.
        let init_msg = json!({
            "type": "m.login.token",
            "token": self.config.access_token,
            "filter": serde_json::from_str::<serde_json::Value>(&filter).ok(),
        });

        write
            .send(Message::Text(init_msg.to_string().into()))
            .await
            .context("failed to send initial sync message")?;

        log::info!("matrix: WebSocket sync started");

        // Listen for incoming events.
        while let Some(msg) = read.next().await {
            match msg.context("WebSocket stream ended")? {
                Message::Text(text) => {
                    if let Err(e) = self.process_ws_event(&text, bus) {
                        log::warn!("matrix: failed to process WebSocket event: {e}");
                    }
                }
                Message::Ping(data) => {
                    if let Err(e) = write.send(Message::Pong(data)).await {
                        log::warn!("matrix: failed to send Pong: {e}");
                        break;
                    }
                }
                Message::Pong(_) => {
                    // Keepalive response received.
                }
                Message::Close(_) | Message::Binary(_) => break,
                _ => {}
            }
        }

        Ok(())
    }

    /// Process a single WebSocket event from the Matrix server.
    fn process_ws_event(&self, raw: &str, bus: &crate::bus::FileBus) -> Result<()> {
        #[derive(Deserialize)]
        struct WsEvent {
            #[serde(rename = "type")]
            event_type: String,
            content: Option<serde_json::Value>,
            room_id: Option<String>,
            sender: Option<String>,
        }

        let event: WsEvent =
            serde_json::from_str(raw).context("failed to parse WebSocket event")?;

        // Only process room messages.
        if event.event_type != "m.room.message" {
            return Ok(());
        }

        let room_id = match event.room_id {
            Some(id) => id,
            None => return Ok(()),
        };

        let sender = event.sender.clone().unwrap_or_else(|| "unknown".to_string());

        // Skip our own messages.
        if sender == self.config.user_id {
            return Ok(());
        }

        let text = event
            .content
            .as_ref()
            .and_then(|c| c.get("body"))
            .and_then(|b| b.as_str())
            .unwrap_or("")
            .to_string();

        if text.is_empty() {
            return Ok(());
        }

        let mx_id = sender
            .strip_prefix('@')
            .unwrap_or(&sender)
            .split(':')
            .next()
            .unwrap_or(&sender)
            .to_string();

        let event =
            crate::bus::BusEvent::new("message", "matrix-websocket", &room_id, &mx_id, &text);

        if let Err(e) = bus.publish(&event) {
            log::warn!("matrix: bus publish failed for {}: {e}", sender);
        } else {
            log::info!("matrix: received message from {} in {}", sender, room_id);
        }

        Ok(())
    }

    /// Fallback: long-poll sync using the Matrix /sync endpoint.
    async fn long_poll_sync(&mut self, bus: &crate::bus::FileBus) -> Result<()> {
        let _filter_name = if self.config.filter.is_some() {
            Some("praxis_filter")
        } else {
            None
        };

        let filter_json = self.config.filter.as_ref().map(|_f| {
            json!({
                "room": {
                    "timeline": { "limit": 20 },
                    "state": { "lazy_load_members": true }
                }
            })
        });

        loop {
            let base_url = format!("{}/_matrix/client/v3/sync", self.config.homeserver);

            let mut url = Url::parse(&base_url).context("invalid homeserver URL")?;
            url.query_pairs_mut()
                .append_pair("access_token", &self.config.access_token)
                .append_pair("timeout", &self.config.sync_timeout_secs.to_string())
                .append_pair("set_presence", "offline");

            if let Some(token) = &self.next_batch {
                url.query_pairs_mut().append_pair("since", token);
            }

            if let Some(filter) = &filter_json {
                url.query_pairs_mut().append_pair("filter", &filter.to_string());
            }

            log::debug!("matrix: polling /sync");

            let response =
                self.http_client.get(url).send().await.context("Matrix /sync request failed")?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                log::warn!("matrix: /sync returned {}: {}", status, body);
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            let sync: SyncResponse =
                response.json().await.context("failed to parse Matrix /sync response")?;

            // Update the next batch token.
            if let Some(next) = sync.next_batch {
                self.next_batch = Some(next);
            }

            // Process room messages.
            if let Some(rooms) = sync.rooms {
                for (room_id, room_data) in rooms.join {
                    for event in room_data.timeline.events {
                        if event.sender.as_deref() == Some(&self.config.user_id) {
                            continue;
                        }

                        let sender_id = event.sender.unwrap_or_else(|| "unknown".to_string());
                        let text = event.content.body;

                        if text.is_empty() {
                            continue;
                        }

                        let bus_event = crate::bus::BusEvent::new(
                            "message",
                            "matrix-sync",
                            &room_id,
                            &sender_id,
                            &text,
                        );

                        if let Err(e) = bus.publish(&bus_event) {
                            log::warn!("matrix: bus publish failed for {}: {e}", sender_id);
                        } else {
                            log::info!("matrix: synced message from {} in {}", sender_id, room_id);
                        }
                    }
                }
            }
        }
    }
}

/// Sync response structure for Matrix long-polling.
#[derive(Debug, Deserialize)]
struct SyncResponse {
    next_batch: Option<String>,
    rooms: Option<SyncRooms>,
}

#[derive(Debug, Deserialize)]
struct SyncRooms {
    #[serde(rename = "join")]
    join: std::collections::HashMap<String, JoinedRoom>,
}

#[derive(Debug, Deserialize)]
struct JoinedRoom {
    timeline: Timeline,
}

#[derive(Debug, Deserialize)]
struct Timeline {
    events: Vec<MatrixEvent>,
}

#[derive(Debug, Deserialize)]
struct MatrixEvent {
    #[serde(rename = "type")]
    event_type: String,
    sender: Option<String>,
    content: MatrixContent,
}

#[derive(Debug, Deserialize)]
struct MatrixContent {
    body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_sync_config_from_env_missing() {
        unsafe { std::env::remove_var("PRAXIS_MATRIX_HOMESERVER") };
        unsafe { std::env::remove_var("PRAXIS_MATRIX_ACCESS_TOKEN") };
        unsafe { std::env::remove_var("PRAXIS_MATRIX_USER_ID") };
        let result = MatrixSyncConfig::from_env();
        assert!(result.is_err());
    }

    #[test]
    fn matrix_sync_config_validate_missing() {
        unsafe { std::env::remove_var("PRAXIS_MATRIX_HOMESERVER") };
        let result = MatrixSyncConfig::validate_environment();
        assert!(result.is_err());
    }

    #[test]
    fn sync_response_deserializes() {
        let json = r#"{
            "next_batch": "s1234",
            "rooms": {
                "join": {
                    "!room:example.com": {
                        "timeline": {
                            "events": [{
                                "type": "m.room.message",
                                "sender": "@user:example.com",
                                "content": { "body": "hello matrix" }
                            }]
                        }
                    }
                }
            }
        }"#;
        let resp: SyncResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_batch, Some("s1234".into()));
        assert!(resp.rooms.is_some());
    }
}
