mod opportunities;
mod sources;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{argus, paths::PraxisPaths, storage::SqliteSessionStore};

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

    Ok(summary)
}

pub fn render_run(summary: &LearningRunSummary) -> String {
    let mut lines = vec![
        "learning: ok".to_string(),
        format!("sources_processed: {}", summary.processed_sources),
        format!("sources_changed: {}", summary.changed_sources),
        format!("opportunities_created: {}", summary.opportunities_created),
        format!("throttle_reached: {}", summary.throttle_reached),
    ];
    if summary.notes.is_empty() {
        lines.push("notes: none".to_string());
    } else {
        lines.push("notes:".to_string());
        lines.extend(summary.notes.iter().map(|note| format!("- {note}")));
    }
    lines.join("\n")
}

pub fn render_list(last_run: Option<StoredLearningRun>, pending: &[StoredOpportunity]) -> String {
    let mut lines = vec!["learning: ready".to_string()];
    if let Some(run) = last_run {
        lines.push(format!(
            "last_run: processed={} changed={} opportunities={} completed_at={}",
            run.processed_sources, run.changed_sources, run.opportunities_created, run.completed_at
        ));
    } else {
        lines.push("last_run: none".to_string());
    }
    lines.push(format!("pending_opportunities: {}", pending.len()));
    if pending.is_empty() {
        lines.push("opportunities: none".to_string());
    } else {
        lines.push("opportunities:".to_string());
        lines.extend(
            pending
                .iter()
                .map(|item| format!("- [{}] {}", item.kind, item.title)),
        );
    }
    lines.join("\n")
}
