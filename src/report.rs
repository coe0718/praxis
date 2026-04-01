use anyhow::Result;
use serde::Serialize;

use crate::{
    config::AppConfig,
    events::{Event, read_events_since},
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{ApprovalStatus, ApprovalStore, QualityStore, SessionStore, SqliteSessionStore},
    tools::{FileToolRegistry, ToolRegistry},
};

#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub instance_name: String,
    pub backend: String,
    pub data_dir: String,
    pub phase: String,
    pub last_outcome: String,
    pub pending_approvals: usize,
    pub registered_tools: usize,
    pub selected_tool: Option<String>,
    pub last_session: Option<LastSessionReport>,
    pub last_review_status: Option<String>,
    pub last_eval: Option<LastEvalReport>,
    pub event_count: usize,
    pub last_event: Option<Event>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastSessionReport {
    pub session_num: i64,
    pub outcome: String,
    pub ended_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastEvalReport {
    pub passed: i64,
    pub failed: i64,
    pub skipped: i64,
    pub trust_failures: i64,
}

pub fn build_status_report(config: &AppConfig, paths: &PraxisPaths) -> Result<StatusReport> {
    let state = SessionState::load(&paths.state_file)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let last_session = store.last_session()?;
    let pending_approvals = store.list_approvals(Some(ApprovalStatus::Pending))?.len();
    let registered_tools = FileToolRegistry.list(paths)?.len();
    let last_review = store.last_review()?;
    let last_eval = store.latest_eval_summary()?;
    let (events, _) = read_events_since(&paths.events_file, 0)?;

    Ok(StatusReport {
        instance_name: config.instance.name.clone(),
        backend: config.agent.backend.clone(),
        data_dir: paths.data_dir.display().to_string(),
        phase: state
            .as_ref()
            .map(|current| current.current_phase.to_string())
            .unwrap_or_else(|| SessionPhase::Sleep.to_string()),
        last_outcome: state
            .as_ref()
            .and_then(|current| current.last_outcome.clone())
            .unwrap_or_else(|| "none".to_string()),
        pending_approvals,
        registered_tools,
        selected_tool: state.and_then(|current| current.selected_tool_name),
        last_session: last_session.map(|session| LastSessionReport {
            session_num: session.session_num,
            outcome: session.outcome,
            ended_at: session.ended_at,
        }),
        last_review_status: last_review.map(|review| review.status.as_str().to_string()),
        last_eval: last_eval.map(|summary| LastEvalReport {
            passed: summary.passed,
            failed: summary.failed,
            skipped: summary.skipped,
            trust_failures: summary.trust_failures,
        }),
        event_count: events.len(),
        last_event: events.last().cloned(),
    })
}

pub fn render_status_report(report: &StatusReport) -> String {
    let mut lines = vec![
        "status: ready".to_string(),
        format!("instance: {}", report.instance_name),
        format!("backend: {}", report.backend),
        format!("data_dir: {}", report.data_dir),
        format!("phase: {}", report.phase),
        format!("last_outcome: {}", report.last_outcome),
        format!("pending_approvals: {}", report.pending_approvals),
        format!("registered_tools: {}", report.registered_tools),
        format!("event_count: {}", report.event_count),
    ];

    if let Some(selected_tool) = &report.selected_tool {
        lines.push(format!("selected_tool: {selected_tool}"));
    }

    if let Some(last_event) = &report.last_event {
        lines.push(format!("last_event: {}", last_event.kind));
    }

    if let Some(session) = &report.last_session {
        lines.push(format!(
            "last_session: #{} {}",
            session.session_num, session.outcome
        ));
        lines.push(format!("last_session_ended_at: {}", session.ended_at));
    } else {
        lines.push("last_session: none".to_string());
    }

    if let Some(review_status) = &report.last_review_status {
        lines.push(format!("last_review: {review_status}"));
    }

    if let Some(eval) = &report.last_eval {
        lines.push(format!(
            "last_eval: passed={} failed={} skipped={} trust_failures={}",
            eval.passed, eval.failed, eval.skipped, eval.trust_failures
        ));
    }

    lines.join("\n")
}
