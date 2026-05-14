use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{config::AppConfig, paths::PraxisPaths, storage::StoredSession};

mod files;
mod git;
mod goals;

pub use files::LocalIdentityPolicy;
pub use files::{DriftReport, FileTier, FileTierPolicy, WritePermission};
pub use git::{AgentManifest, GitIdentity, PersonalityTraits};
pub use goals::{MarkdownGoalParser, ensure_goal};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub line_number: usize,
    pub parent_id: Option<String>,
    pub blocked_by: Vec<String>,
    pub wake_when: Option<String>,
}

pub trait GoalParser {
    fn parse_goals(&self, content: &str) -> Result<Vec<Goal>>;
    fn load_goals(&self, path: &Path) -> Result<Vec<Goal>>;
}

pub trait IdentityPolicy {
    fn ensure_foundation(
        &self,
        paths: &PraxisPaths,
        config: &AppConfig,
        now: DateTime<Utc>,
    ) -> Result<()>;
    fn validate(&self, paths: &PraxisPaths) -> Result<()>;
    fn append_journal(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()>;
    fn append_metrics(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()>;
    fn read_day_count(&self, paths: &PraxisPaths) -> Result<i64>;
    /// Check whether writing to the given path is allowed.
    fn check_write_permission(
        &self,
        paths: &PraxisPaths,
        relative_path: &str,
    ) -> Result<WritePermission> {
        let _ = (paths, relative_path);
        Ok(WritePermission::Allowed)
    }
    /// Detect identity drift (locked file modifications).
    fn detect_drift(&self, paths: &PraxisPaths) -> Result<crate::identity::DriftReport> {
        let _ = paths;
        Ok(crate::identity::DriftReport::default())
    }
}
