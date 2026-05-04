use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(super) struct ApprovalRow {
    pub id: i64,
    pub tool_name: String,
    pub summary: String,
    pub requested_by: String,
    pub write_paths: Vec<String>,
    pub payload_json: Option<String>,
    pub status: String,
    pub status_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub(super) struct MemoryRow {
    pub id: i64,
    pub tier: String,
    pub content: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub score: f64,
    pub memory_type: String,
}

#[derive(Serialize)]
pub(super) struct GoalRow {
    pub raw_id: String,
    pub title: String,
    pub completed: bool,
}

#[derive(Deserialize)]
pub(super) struct AddGoalBody {
    pub description: String,
}

#[derive(Deserialize)]
pub(super) struct WakeBody {
    pub task: Option<String>,
    pub reason: Option<String>,
    pub urgent: Option<bool>,
}

#[derive(Deserialize)]
pub(super) struct RunBody {
    pub task: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct WriteFileBody {
    pub content: String,
}

#[derive(Deserialize)]
pub(super) struct AskBody {
    pub prompt: String,
}

#[derive(Deserialize)]
pub(super) struct BoundaryAddBody {
    pub rule: String,
}

#[derive(Deserialize)]
pub(super) struct BoundaryConfirmBody {
    pub note: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct LearningNoteBody {
    pub text: String,
}

#[derive(Deserialize)]
pub(super) struct AgentsAddBody {
    pub section: String,
    pub note: String,
}

#[derive(Deserialize)]
pub(super) struct VaultSetBody {
    pub name: String,
    pub kind: String,
    pub value: Option<String>,
    pub env: Option<String>,
    pub fallback: Option<String>,
}

// ── Chat completions (OpenAI-compatible) ──────────────────────────────────────

#[derive(Deserialize)]
pub(super) struct ChatCompletionsRequest {
    pub model: Option<String>,
    pub messages: Vec<ApiChatMessage>,
    #[serde(default = "default_max_tokens")]
    pub max_completion_tokens: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
}

fn default_max_tokens() -> Option<u32> {
    Some(1024)
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiChatMessage {
    pub role: String,
    pub content: ApiChatContent,
}

/// Message content: plain text string OR array of content blocks.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub(super) enum ApiChatContent {
    Text(String),
    Blocks(Vec<ApiContentBlock>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum ApiContentBlock {
    Text { text: String },
    ImageUrl { image_url: ApiImageUrl },
}

#[derive(Deserialize, Debug, Clone)]
pub(super) struct ApiImageUrl {
    pub url: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ChatCompletionsResponse {
    pub id: String,
    pub object: &'static str,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

#[derive(Serialize)]
pub(super) struct ChatChoice {
    pub index: u32,
    pub message: ChatResponseMessage,
    pub finish_reason: &'static str,
}

#[derive(Serialize)]
pub(super) struct ChatResponseMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Serialize)]
pub(super) struct ChatUsage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}
