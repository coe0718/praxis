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
