use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod anatomy;
mod ops;
mod provider;
mod sqlite;

pub use anatomy::AnatomyStore;
pub use ops::{OperationalMemoryCounts, OperationalMemoryStore};
pub use provider::ProviderUsageStore;
pub use sqlite::SqliteSessionStore;

pub trait SessionStore {
    fn initialize(&self) -> Result<()>;
    fn validate_schema(&self) -> Result<()>;
    fn record_session(&self, record: &SessionRecord) -> Result<StoredSession>;
    fn last_session(&self) -> Result<Option<StoredSession>>;
}

pub trait QualityStore {
    fn update_session_quality(&self, session_id: i64, update: &SessionQualityUpdate) -> Result<()>;
    fn record_review(&self, record: &ReviewRecord) -> Result<StoredReview>;
    fn last_review(&self) -> Result<Option<StoredReview>>;
    fn record_eval_run(&self, record: &EvalRunRecord) -> Result<()>;
    fn latest_eval_summary(&self) -> Result<Option<StoredEvalSummary>>;
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
    pub repeated_reads_avoided: i64,
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

#[derive(Debug, Clone)]
pub struct SessionQualityUpdate {
    pub outcome: String,
    pub action_summary: String,
    pub reviewer_passes: i64,
    pub reviewer_failures: i64,
    pub eval_passes: i64,
    pub eval_failures: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    Passed,
    Failed,
    Skipped,
}

impl ReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewStatus::Passed => "passed",
            ReviewStatus::Failed => "failed",
            ReviewStatus::Skipped => "skipped",
        }
    }

    pub fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            _ => Err(format!("unknown review status {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReviewRecord {
    pub session_id: i64,
    pub goal_id: Option<String>,
    pub status: ReviewStatus,
    pub summary: String,
    pub findings_json: String,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReview {
    pub id: i64,
    pub session_id: i64,
    pub goal_id: Option<String>,
    pub status: ReviewStatus,
    pub summary: String,
    pub findings_json: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalStatus {
    Passed,
    Failed,
    Skipped,
}

impl EvalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvalStatus::Passed => "passed",
            EvalStatus::Failed => "failed",
            EvalStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvalSeverity {
    Cosmetic,
    Functional,
    TrustDamaging,
}

impl EvalSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvalSeverity::Cosmetic => "cosmetic",
            EvalSeverity::Functional => "functional",
            EvalSeverity::TrustDamaging => "trust_damaging",
        }
    }
}

#[derive(Debug, Clone)]
pub struct EvalRunRecord {
    pub session_id: i64,
    pub eval_id: String,
    pub eval_name: String,
    pub status: EvalStatus,
    pub severity: EvalSeverity,
    pub summary: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEvalSummary {
    pub session_id: i64,
    pub passed: i64,
    pub failed: i64,
    pub skipped: i64,
    pub trust_failures: i64,
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
    pub payload_json: Option<String>,
    pub status: ApprovalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredApprovalRequest {
    pub id: i64,
    pub tool_name: String,
    pub summary: String,
    pub requested_by: String,
    pub write_paths: Vec<String>,
    pub payload_json: Option<String>,
    pub status: ApprovalStatus,
    pub status_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
