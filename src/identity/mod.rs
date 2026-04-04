use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{config::AppConfig, paths::PraxisPaths, storage::StoredSession};

mod files;
mod goals;

pub use files::LocalIdentityPolicy;
pub use goals::{MarkdownGoalParser, ensure_goal};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub line_number: usize,
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
}
