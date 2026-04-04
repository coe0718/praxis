mod opportunities;
mod proposals;
mod render;
mod sources;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{argus, paths::PraxisPaths, storage::SqliteSessionStore};

pub use render::{render_action, render_list, render_run};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredLearningSource {
    pub path: String,
    pub last_modified_at: String,
    pub byte_len: i64,
    pub summary: String,
    pub last_processed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLearningSourceState {
    pub path: String,
    pub last_modified_at: String,
    pub byte_len: i64,
    pub summary: String,
    pub last_processed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredLearningRun {
    pub id: i64,
    pub processed_sources: i64,
    pub changed_sources: i64,
    pub opportunities_created: i64,
    pub notes: Vec<String>,
    pub completed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewLearningRun {
    pub processed_sources: i64,
    pub changed_sources: i64,
    pub opportunities_created: i64,
    pub notes: Vec<String>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpportunityStatus {
    Pending,
    Accepted,
    Dismissed,
}

impl OpportunityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn heading(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Accepted => "Accepted",
            Self::Dismissed => "Dismissed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredOpportunity {
    pub id: i64,
    pub signature: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityCandidate {
    pub signature: String,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub evidence_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningRunSummary {
    pub processed_sources: usize,
    pub changed_sources: usize,
    pub opportunities_created: usize,
    pub throttle_reached: bool,
    pub notes: Vec<String>,
}

pub fn run_once(
    paths: &PraxisPaths,
    store: &SqliteSessionStore,
    now: DateTime<Utc>,
) -> Result<LearningRunSummary> {
    let source_result = sources::process_sources(paths, store, now)?;
    let argus_report = argus::analyze(&paths.database_file, 10)?;
    let mining_result = opportunities::mine(paths, store, &argus_report, now)?;
    let mut notes = source_result.notes;
    notes.extend(mining_result.notes);

    let summary = LearningRunSummary {
        processed_sources: source_result.processed_sources,
        changed_sources: source_result.changed_sources,
        opportunities_created: mining_result.created,
        throttle_reached: mining_result.throttle_reached,
        notes,
    };

    store.record_learning_run(NewLearningRun {
        processed_sources: summary.processed_sources as i64,
        changed_sources: summary.changed_sources as i64,
        opportunities_created: summary.opportunities_created as i64,
        notes: summary.notes.clone(),
        completed_at: now,
    })?;
    proposals::sync(paths, store)?;

    Ok(summary)
}

pub fn update_opportunity(
    paths: &PraxisPaths,
    store: &SqliteSessionStore,
    id: i64,
    status: OpportunityStatus,
    now: DateTime<Utc>,
) -> Result<Option<StoredOpportunity>> {
    let updated = store.set_opportunity_status(id, status, now)?;
    proposals::sync(paths, store)?;
    Ok(updated)
}
