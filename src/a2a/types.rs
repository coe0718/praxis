//! Agent-to-Agent (A2A) protocol types.
//!
//! Based on Google's A2A spec draft — standardises how autonomous agents
//! discover, negotiate, and collaborate across process boundaries.
//!
//! Praxis implements both the client (outbound delegation) and server
//! (inbound task acceptance) sides.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ── Agent Card ───────────────────────────────────────────────────────────────

/// Public metadata an agent exposes for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub url: String,
    pub version: String,
    pub capabilities: AgentCapabilities,
    pub authentication: AuthScheme,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentCapabilities {
    pub streaming: bool,
    pub push_notifications: bool,
    pub state_transition_history: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthScheme {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "api_key")]
    ApiKey { location: String },
    #[serde(rename = "oauth2")]
    OAuth2 { authority: String },
}

// ── Task Lifecycle ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub session_id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Artifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<TaskStatusUpdate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(PartialEq)]
pub enum TaskStatus {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatusUpdate {
    pub state: TaskStatus,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "file")]
    File { name: String, mime_type: String, bytes: Option<String>, uri: Option<String> },
    #[serde(rename = "data")]
    Data { data: Value },
}

// ── Artifact ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub parts: Vec<Part>,
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chunk: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

// ── Request / Response ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTaskRequest {
    pub id: String,
    pub session_id: String,
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTaskResponse {
    pub id: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact: Option<Artifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskRequest {
    pub id: String,
    pub history_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskResponse {
    pub id: String,
    pub status: TaskStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_roundtrip() {
        let t = TaskStatus::Working;
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, "\"working\"");
        let back: TaskStatus = serde_json::from_str(&s).unwrap();
        assert_eq!(back, TaskStatus::Working);
    }

    #[test]
    fn agent_card_serializes() {
        let card = AgentCard {
            name: "praxis".to_string(),
            description: "Autonomous coding agent".to_string(),
            url: "http://localhost:3000".to_string(),
            version: "0.1.0".to_string(),
            capabilities: AgentCapabilities {
                streaming: false,
                push_notifications: false,
                state_transition_history: true,
            },
            authentication: AuthScheme::None,
        };
        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("praxis"));
    }
}
