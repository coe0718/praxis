use anyhow::Result;
use chrono::{DateTime, Utc};

mod sqlite;

pub use sqlite::SqliteSessionStore;

pub trait SessionStore {
    fn initialize(&self) -> Result<()>;
    fn validate_schema(&self) -> Result<()>;
    fn record_session(&self, record: &SessionRecord) -> Result<StoredSession>;
    fn last_session(&self) -> Result<Option<StoredSession>>;
}

pub trait ApprovalStore {
    fn queue_approval(&self, request: &NewApprovalRequest) -> Result<StoredApprovalRequest>;
    fn list_approvals(&self, status: Option<ApprovalStatus>) -> Result<Vec<StoredApprovalRequest>>;
    fn get_approval(&self, id: i64) -> Result<Option<StoredApprovalRequest>>;
    fn set_approval_status(
        &self,
        id: i64,
        status: ApprovalStatus,
        note: Option<&str>,
    ) -> Result<Option<StoredApprovalRequest>>;
    fn next_approved_request(&self) -> Result<Option<StoredApprovalRequest>>;
    fn mark_approval_consumed(&self, id: i64) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub day: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub outcome: String,
    pub selected_goal_id: Option<String>,
    pub selected_goal_title: Option<String>,
    pub selected_task: Option<String>,
    pub action_summary: String,
    pub phase_durations_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSession {
    pub id: i64,
    pub day: i64,
    pub session_num: i64,
    pub started_at: String,
    pub ended_at: String,
    pub outcome: String,
    pub selected_goal_id: Option<String>,
    pub selected_goal_title: Option<String>,
    pub selected_task: Option<String>,
    pub action_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Executed,
}

impl ApprovalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalStatus::Pending => "pending",
            ApprovalStatus::Approved => "approved",
            ApprovalStatus::Rejected => "rejected",
            ApprovalStatus::Executed => "executed",
        }
    }

    pub fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "executed" => Ok(Self::Executed),
            _ => Err(format!("unknown approval status {value}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewApprovalRequest {
    pub tool_name: String,
    pub summary: String,
    pub requested_by: String,
    pub write_paths: Vec<String>,
    pub status: ApprovalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredApprovalRequest {
    pub id: i64,
    pub tool_name: String,
    pub summary: String,
    pub requested_by: String,
    pub write_paths: Vec<String>,
    pub status: ApprovalStatus,
    pub status_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
