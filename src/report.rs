mod render;

use anyhow::Result;
use serde::Serialize;

use crate::{
    argus::analyze,
    config::AppConfig,
    events::{Event, read_events_since},
    learning::StoredLearningRun,
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{
        AnatomyStore, ApprovalStatus, ApprovalStore, OperationalMemoryStore, ProviderUsageStore,
        QualityStore, SessionStore, SqliteSessionStore,
    },
    tools::{FileToolRegistry, ToolRegistry},
};

pub use render::render_status_report;

#[derive(Debug, Clone, Serialize)]
pub struct StatusReport {
    pub instance_name: String,
    pub backend: String,
    pub data_dir: String,
    pub phase: String,
    pub last_outcome: String,
    pub repeated_reads_avoided: u32,
    pub anatomy_entries: i64,
    pub pending_approvals: usize,
    pub pending_opportunities: i64,
    pub registered_tools: usize,
    pub operational_memory: OperationalMemoryReport,
    pub selected_tool: Option<String>,
    pub last_session: Option<LastSessionReport>,
    pub last_review_status: Option<String>,
    pub last_eval: Option<LastEvalReport>,
    pub last_provider_usage: Option<LastProviderUsageReport>,
    pub last_token_summary: Option<LastTokenSummaryReport>,
    pub last_token_hotspot: Option<LastTokenHotspotReport>,
    pub last_learning_run: Option<StoredLearningRun>,
    pub drift_status: String,
    pub event_count: usize,
    pub last_event: Option<Event>,
    pub heartbeat: Option<HeartbeatReport>,
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

#[derive(Debug, Clone, Serialize)]
pub struct LastProviderUsageReport {
    pub attempt_count: i64,
    pub failure_count: i64,
    pub last_provider: String,
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OperationalMemoryReport {
    pub do_not_repeat: i64,
    pub known_bugs: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastTokenSummaryReport {
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastTokenHotspotReport {
    pub phase: String,
    pub provider: String,
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatReport {
    pub phase: String,
    pub updated_at: String,
}

pub fn build_status_report(config: &AppConfig, paths: &PraxisPaths) -> Result<StatusReport> {
    let state = SessionState::load(&paths.state_file)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let last_session = store.last_session()?;
    let anatomy_entries = store.anatomy_entry_count()?;
    let pending_approvals = store.list_approvals(Some(ApprovalStatus::Pending))?.len();
    let pending_opportunities = store.pending_opportunity_count()?;
    let registered_tools = FileToolRegistry.list(paths)?.len();
    let operational_memory = store.operational_memory_counts()?;
    let last_review = store.last_review()?;
    let last_eval = store.latest_eval_summary()?;
    let last_provider_usage = store.latest_provider_usage()?;
    let last_token_summary = store.latest_token_summary()?;
    let last_token_hotspot = store.latest_phase_token_usage(1)?.into_iter().next();
    let last_learning_run = store.latest_learning_run()?;
    let drift_status = analyze(&paths.database_file, 10)?
        .drift
        .status
        .as_str()
        .to_string();
    let heartbeat = crate::heartbeat::read_heartbeat(&paths.heartbeat_file).ok();
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
        repeated_reads_avoided: state
            .as_ref()
            .map(|current| current.repeated_reads_avoided)
            .unwrap_or_default(),
        anatomy_entries,
        pending_approvals,
        pending_opportunities,
        registered_tools,
        operational_memory: OperationalMemoryReport {
            do_not_repeat: operational_memory.do_not_repeat,
            known_bugs: operational_memory.known_bugs,
        },
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
        last_provider_usage: last_provider_usage.map(|usage| LastProviderUsageReport {
            attempt_count: usage.attempt_count,
            failure_count: usage.failure_count,
            last_provider: usage.last_provider,
            tokens_used: usage.tokens_used,
            estimated_cost_micros: usage.estimated_cost_micros,
        }),
        last_token_summary: last_token_summary.map(|summary| LastTokenSummaryReport {
            tokens_used: summary.tokens_used,
            estimated_cost_micros: summary.estimated_cost_micros,
        }),
        last_token_hotspot: last_token_hotspot.map(|usage| LastTokenHotspotReport {
            phase: usage.phase,
            provider: usage.provider,
            tokens_used: usage.tokens_used,
            estimated_cost_micros: usage.estimated_cost_micros,
        }),
        last_learning_run,
        drift_status,
        event_count: events.len(),
        last_event: events.last().cloned(),
        heartbeat: heartbeat.map(|record| HeartbeatReport {
            phase: record.phase,
            updated_at: record.updated_at,
        }),
    })
}
