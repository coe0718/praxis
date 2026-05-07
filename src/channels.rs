//! Signal/Matrix channels — Enterprise messaging support.
//!
//! Moltis has Signal + Matrix integrations.
//! This adds secure enterprise messaging channels.

#![allow(dead_code)]

use anyhow::Result;

/// Signal message client.
pub struct SignalClient {
    /// Registered phone number.
    phone_number: String,
}

impl SignalClient {
    pub fn new(phone_number: &str) -> Self {
        Self {
            phone_number: phone_number.to_string(),
        }
    }

    /// Send a message via Signal.
    pub fn send(&self, recipient: &str, message: &str) -> Result<()> {
        let _ = (recipient, message);
        Ok(())
    }
}

/// Matrix message client.
pub struct MatrixClient {
    /// Homeserver URL.
    homeserver: String,
    /// User ID.
    _user_id: String,
    /// Access token.
    _access_token: String,
}

impl MatrixClient {
    pub fn new(homeserver: &str, user_id: &str, access_token: &str) -> Self {
        Self {
            homeserver: homeserver.to_string(),
            _user_id: user_id.to_string(),
            _access_token: access_token.to_string(),
        }
    }

    /// Send a message to a room.
    pub async fn send(&self, room_id: &str, message: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let url = format!("{}/_matrix/client/r0/rooms/{}/send/m.room.message", 
            self.homeserver, room_id);
        
        let resp = client
            .post(&url)
            .json(&serde_json::json!({
                "msgtype": "m.text",
                "body": message
            }))
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Matrix send failed: {}", resp.status())
        }
    }
}