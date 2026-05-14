//! WhatsApp webhook receiver — inbound message handling for WhatsApp Business API.
//!
//! WhatsApp Cloud API sends inbound messages via webhooks. This module
//! receives and dispatches them to the Praxis message bus.
//!
//! Set `PRAXIS_WHATSAPP_WEBHOOK_VERIFY_TOKEN` for verification.
use axum::{
    Json,
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

/// WhatsApp webhook verification query parameters.
#[derive(Deserialize)]
pub struct WhatsappVerifyQuery {
    pub hub_mode: String,
    pub hub_verify_token: String,
    pub hub_challenge: String,
}

/// WhatsApp webhook message payload.
#[derive(Deserialize, Debug)]
pub struct WhatsappWebhookPayload {
    pub object: String,
    pub entry: Vec<WhatsappEntry>,
}

#[derive(Deserialize, Debug)]
pub struct WhatsappEntry {
    pub id: String,
    pub changes: Vec<WhatsappChange>,
}

#[derive(Deserialize, Debug)]
pub struct WhatsappChange {
    pub value: WhatsappValue,
    pub field: String,
}

#[derive(Deserialize, Debug)]
pub struct WhatsappValue {
    pub messaging_product: String,
    pub sender_id: Option<String>,
    pub message_id: Option<String>,
    pub phone_number_id: Option<String>,
    pub text: Option<WhatsappTextContent>,
    #[serde(rename = "type")]
    pub message_type: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct WhatsappTextContent {
    pub body: String,
}

/// Handle WhatsApp webhook verification (GET).
pub async fn whatsapp_verify(
    State(state): State<crate::dashboard::server::DashboardState>,
    Query(params): Query<WhatsappVerifyQuery>,
) -> impl IntoResponse {
    let expected_token = state.whatsapp_verify_token.as_deref();

    match expected_token {
        Some(token) if token == params.hub_verify_token => {
            (StatusCode::OK, params.hub_challenge.clone()).into_response()
        }
        None => {
            log::warn!("whatsapp webhook: no PRAXIS_WHATSAPP_WEBHOOK_VERIFY_TOKEN configured");
            StatusCode::UNAUTHORIZED.into_response()
        }
        _ => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// Handle WhatsApp inbound messages (POST).
pub async fn whatsapp_inbound(
    State(state): State<crate::dashboard::server::DashboardState>,
    body: Bytes,
) -> impl IntoResponse {
    use crate::bus::{BusEvent, FileBus, MessageBus};

    let payload: WhatsappWebhookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("whatsapp webhook: invalid JSON body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let bus = FileBus::new(state.data_dir.join("bus.jsonl"));

    for entry in &payload.entry {
        for change in &entry.changes {
            if change.field != "messages" {
                continue;
            }

            let value = &change.value;
            let sender = value.sender_id.clone().unwrap_or_else(|| "unknown".to_string());
            let text = value.text.as_ref().map(|t| t.body.clone()).unwrap_or_default();

            if text.is_empty() {
                continue;
            }

            let phone_id = value.phone_number_id.clone().unwrap_or_default();

            let event =
                BusEvent::new("message", "whatsapp-webhook", &phone_id, sender.clone(), &text);

            if let Err(e) = bus.publish(&event) {
                log::warn!("whatsapp webhook: bus publish failed for {}: {e}", sender);
            } else {
                log::info!("whatsapp webhook: inbound message from {} in {}", sender, phone_id);
            }
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whatsapp_payload_deserializes() {
        let json = r#"{
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "10234567",
                "changes": [{
                    "value": {
                        "messaging_product": "whatsapp",
                        "sender_id": "15551234567",
                        "phone_number_id": "1234567890",
                        "text": { "body": "hello from whatsapp" }
                    },
                    "field": "messages"
                }]
            }]
        }"#;

        let payload: WhatsappWebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.entry.len(), 1);
        let msg = &payload.entry[0].changes[0].value;
        assert_eq!(msg.sender_id.as_deref(), Some("15551234567"));
        assert_eq!(msg.text.as_ref().unwrap().body, "hello from whatsapp");
    }
}
