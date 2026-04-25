mod drift;
mod patterns;
mod render;

use std::path::Path;

use anyhow::{Context, Result};
use drift::{DriftReport, detect_drift};
use patterns::{
    RepeatedWorkPattern, cluster_failures, recent_sessions, repeated_work_patterns, token_hotspots,
};
use rusqlite::Connection;

pub use drift::DriftStatus;
pub use render::render;

#[derive(Debug, Clone, PartialEq)]
pub struct ArgusReport {
    pub session_count: usize,
    pub review_failures: i64,
    pub eval_failures: i64,
    pub loop_guard_blocks: i64,
    pub waiting_sessions: usize,
    pub repeated_reads_avoided: i64,
    pub drift: DriftReport,
    pub repeated_work: Vec<RepeatedWorkPattern>,
    pub failure_clusters: Vec<(String, usize)>,
    pub token_hotspots: Vec<(String, String, i64)>,
    pub directives: Vec<String>,
}

pub fn analyze(database_file: &Path, limit: usize) -> Result<ArgusReport> {
    let connection = Connection::open(database_file)
        .with_context(|| format!("failed to open {}", database_file.display()))?;
    let sessions = recent_sessions(&connection, limit.max(1))?;
    let session_count = sessions.len();
    let review_failures = sessions.iter().map(|session| session.reviewer_failures).sum();
    let eval_failures = sessions.iter().map(|session| session.eval_failures).sum();
    let loop_guard_blocks = sessions
        .iter()
        .filter(|session| session.outcome == "blocked_loop_guard")
        .count() as i64;
    let waiting_sessions = sessions
        .iter()
        .filter(|session| session.outcome == "waiting_on_dependencies")
        .count();
    let repeated_reads_avoided =
        sessions.iter().map(|session| session.repeated_reads_avoided).sum();
    let drift = detect_drift(&sessions, 5);
    let repeated_work = repeated_work_patterns(&sessions);
    let failure_clusters = cluster_failures(&sessions);
    let token_hotspots = token_hotspots(&connection, limit.max(1))?;
    let directives = render::directives(render::DirectiveInputs {
        session_count,
        review_failures,
        eval_failures,
        loop_guard_blocks,
        waiting_sessions,
        repeated_reads_avoided,
        drift: &drift,
        repeated_work: &repeated_work,
        token_hotspots: &token_hotspots,
    });

    Ok(ArgusReport {
        session_count,
        review_failures,
        eval_failures,
        loop_guard_blocks,
        waiting_sessions,
        repeated_reads_avoided,
        drift,
        repeated_work,
        failure_clusters,
        token_hotspots,
        directives,
    })
}
