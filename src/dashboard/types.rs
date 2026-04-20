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
