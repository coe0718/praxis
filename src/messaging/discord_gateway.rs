//! Discord Gateway WebSocket client for real-time push delivery.
//!
//! Uses `tokio-tungstenite` to connect to `wss://gateway.discord.gg` and receives
//! MESSAGE_CREATE events in real-time, publishing them to the message bus.
//!
//! # Setup
//! Set `PRAXIS_DISCORD_BOT_TOKEN` (bot token with `MESSAGE_CONTENT_INTENT` enabled)
//! and optionally `PRAXIS_DISCORD_GATEWAY_INTENTS` (bitwise OR of intent flags, default 33550336).

use std::time::Duration;

use anyhow::{bail, Context, Result};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::bus::{BusEvent, MessageBus};

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Discord gateway opcodes.
#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
}

impl TryFrom<u8> for Opcode {
    type Error = anyhow::Error;
    fn try_from(v: u8) -> Result<Self> {
        match v {
            0 => Ok(Opcode::Dispatch),
            1 => Ok(Opcode::Heartbeat),
            2 => Ok(Opcode::Identify),
            3 => Ok(Opcode::PresenceUpdate),
            4 => Ok(Opcode::VoiceStateUpdate),
            6 => Ok(Opcode::Resume),
            7 => Ok(Opcode::Reconnect),
            9 => Ok(Opcode::InvalidSession),
            10 => Ok(Opcode::Hello),
            11 => Ok(Opcode::HeartbeatAck),
            _ => bail!("unknown opcode: {v}"),
        }
    }
}

impl From<Opcode> for u8 {
    fn from(op: Opcode) -> Self {
        op as u8
    }
}

/// Gateway hello payload.
#[derive(Debug, Deserialize)]
struct HelloData {
    heartbeat_interval: u64,
}

/// Gateway message wrapper.
#[derive(Debug, Deserialize)]
struct GatewayMessage<T = serde_json::Value> {
    op: u8,
    t: Option<String>,
    d: T,
}

/// MESSAGE_CREATE event data.
#[derive(Debug, Deserialize)]
struct MessageCreate {
    #[serde(rename = "channel_id")]
    channel_id: String,
    content: String,
    author: MessageAuthor,
}

#[derive(Debug, Deserialize)]
struct MessageAuthor {
    #[serde(rename = "id")]
    id: String,
}

/// Run the Discord Gateway client in a loop, reconnecting on errors.
pub async fn run_gateway(bus: std::sync::Arc<dyn MessageBus + Send + Sync>, token: String) {
    loop {
        match run_gateway_once(bus.clone(), &token).await {
            Ok(_) => {
                log::info!("discord gateway: connection closed, reconnecting in 5s");
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                log::warn!("discord gateway error: {e}, reconnecting in 10s");
                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}

async fn run_gateway_once(bus: std::sync::Arc<dyn MessageBus + Send + Sync>, token: &str) -> Result<()> {
    log::info!("discord gateway: connecting to {GATEWAY_URL}");
    let (ws_stream, _) = connect_async(GATEWAY_URL).await.context("gateway connection failed")?;
    log::info!("discord gateway: connected");

    let (write, read) = ws_stream.split();
    let intents = parse_intents();

    // Channel for heartbeat sender
    let (hb_tx, mut hb_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Wait for hello
    let mut read = read;
    let msg = read.next().await.context("expected hello")??;
    let msg: GatewayMessage<HelloData> = serde_json::from_str(&msg.to_string())?;
    if msg.op != Opcode::Hello as u8 {
        bail!("expected opcode 10 (hello), got {}", msg.op);
    }
    let heartbeat_ms = msg.d.heartbeat_interval.max(40000).max(45000);

    // Spawn heartbeat task
    tokio::spawn(async move {
        let mut heartbeat_interval = interval(Duration::from_millis(heartbeat_ms));
        loop {
            heartbeat_interval.tick().await;
            // Notify sender that we need to send a heartbeat
            if hb_tx.send(()).is_err() {
                break;
            }
        }
    });

    // Spawn sender task
    let mut write = write;
    let token_clone = token.to_string();
    tokio::spawn(async move {
        // Send identify immediately
        let identify_msg = serde_json::json!({
            "op": Opcode::Identify as u8,
            "d": {
                "token": token_clone,
                "intents": intents,
                "properties": {
                    "$os": "linux",
                    "$browser": "praxis",
                    "$device": "praxis"
                }
            }
        });
        let _ = write.send(Message::text(identify_msg.to_string())).await;

        // Handle heartbeats
        while hb_rx.recv().await.is_some() {
            let hb_msg = serde_json::json!({
                "op": Opcode::Heartbeat as u8,
                "d": serde_json::Value::Null
            });
            if write.send(Message::text(hb_msg.to_string())).await.is_err() {
                break;
            }
        }
    });

    // Read messages - process MESSAGE_CREATE events
    while let Some(Ok(msg)) = read.next().await {
        if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(&msg.to_string())
            && gateway_msg.t.as_deref() == Some("MESSAGE_CREATE")
            && let Ok(msg_data) = serde_json::from_value::<MessageCreate>(gateway_msg.d)
        {
            let event = BusEvent::new(
                "message",
                "discord-gateway",
                &msg_data.channel_id,
                msg_data.author.id.clone(),
                &msg_data.content,
            );
            match bus.publish(&event) {
                Err(e) => log::warn!("discord gateway bus publish failed: {e}"),
                Ok(()) => log::debug!(
                    "discord gateway: message from {}: {}",
                    msg_data.channel_id,
                    msg_data.content
                ),
            }
        }
    }

    Ok(())
}

/// Parse intents from env var, default to MESSAGE_CONTENT_INTENT + GUILD_MESSAGES.
fn parse_intents() -> u32 {
    std::env::var("PRAXIS_DISCORD_GATEWAY_INTENTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(33550336)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_opcode_conversion() {
        assert_eq!(Opcode::Hello as u8, 10);
        assert_eq!(Opcode::Heartbeat as u8, 1);
    }
}