use chrono::TimeZone;
use tempfile::tempdir;

use super::{PraxisRuntime, RunOptions, StubBackend};
use crate::{
    config::AppConfig,
    events::NoopEventSink,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{
        ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionStore, SqliteSessionStore,
    },
    time::FixedClock,
    tools::{FileToolRegistry, LoopGuard, ToolRegistry},
};

#[test]
fn runtime_blocks_repeated_tool_invocations() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let identity = LocalIdentityPolicy;
    let started_at = chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap();

    identity
        .ensure_foundation(&paths, &config, started_at)
        .unwrap();
    FileToolRegistry.ensure_foundation(&paths).unwrap();

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize().unwrap();
    let approved = store
        .queue_approval(&NewApprovalRequest {
            tool_name: "praxis-data-write".to_string(),
            summary: "Update JOURNAL.md".to_string(),
            requested_by: "operator".to_string(),
            write_paths: vec!["JOURNAL.md".to_string()],
            payload_json: Some("{\"append_text\":\"runtime wrote this\"}".to_string()),
            status: ApprovalStatus::Approved,
        })
        .unwrap();

    let mut state = tool_state(started_at, approved.id);
    let invocation =
        "praxis-data-write|Update JOURNAL.md|JOURNAL.md|{\"append_text\":\"runtime wrote this\"}";
    LoopGuard.record(&mut state, invocation, 3);
    LoopGuard.record(&mut state, invocation, 3);
    state.save(&paths.state_file).unwrap();

    let summary = run_once(&paths, &config, &identity, &store, started_at);

    assert_eq!(summary.outcome, "blocked_loop_guard");
    assert_eq!(
        store.get_approval(approved.id).unwrap().unwrap().status,
        ApprovalStatus::Approved
    );
}

#[test]
fn runtime_blocks_repeated_two_step_tool_patterns() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let identity = LocalIdentityPolicy;
    let started_at = chrono::Utc.with_ymd_and_hms(2026, 4, 1, 12, 0, 0).unwrap();

    identity
        .ensure_foundation(&paths, &config, started_at)
        .unwrap();
    FileToolRegistry.ensure_foundation(&paths).unwrap();

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize().unwrap();
    store
        .queue_approval(&NewApprovalRequest {
            tool_name: "praxis-data-write".to_string(),
            summary: "Write journal variant A".to_string(),
            requested_by: "operator".to_string(),
            write_paths: vec!["JOURNAL.md".to_string()],
            payload_json: Some("{\"append_text\":\"variant-a\"}".to_string()),
            status: ApprovalStatus::Approved,
        })
        .unwrap();
    let approved = store
        .queue_approval(&NewApprovalRequest {
            tool_name: "praxis-data-write".to_string(),
            summary: "Write journal variant B".to_string(),
            requested_by: "operator".to_string(),
            write_paths: vec!["JOURNAL.md".to_string()],
            payload_json: Some("{\"append_text\":\"variant-b\"}".to_string()),
            status: ApprovalStatus::Approved,
        })
        .unwrap();

    let a = "praxis-data-write|Write journal variant A|JOURNAL.md|{\"append_text\":\"variant-a\"}";
    let b = "praxis-data-write|Write journal variant B|JOURNAL.md|{\"append_text\":\"variant-b\"}";
    let mut state = tool_state(started_at, approved.id);
    for invocation in [a, b, a, b, a] {
        LoopGuard.record(&mut state, invocation, 3);
    }
    state.save(&paths.state_file).unwrap();

    let summary = run_once(&paths, &config, &identity, &store, started_at);

    assert_eq!(summary.outcome, "blocked_loop_guard");
    assert!(
        summary
            .action_summary
            .contains("repeating 2-step tool pattern")
    );
}

fn tool_state(started_at: chrono::DateTime<chrono::Utc>, request_id: i64) -> SessionState {
    SessionState {
        current_phase: SessionPhase::Act,
        started_at,
        updated_at: started_at,
        completed_at: None,
        selected_goal_id: None,
        selected_goal_title: None,
        requested_task: None,
        orientation_summary: Some("Loaded prior state.".to_string()),
        action_summary: Some("Resume approved tool request.".to_string()),
        last_outcome: Some("approved_tool_selected".to_string()),
        resume_count: 0,
        selected_tool_name: Some("praxis-data-write".to_string()),
        selected_tool_request_id: Some(request_id),
        tool_invocation_hashes: Vec::new(),
        provider_attempts: Vec::new(),
        file_reads: Vec::new(),
        repeated_reads_avoided: 0,
        context_sources: Vec::new(),
    }
}

fn run_once(
    paths: &PraxisPaths,
    config: &AppConfig,
    identity: &LocalIdentityPolicy,
    store: &SqliteSessionStore,
    started_at: chrono::DateTime<chrono::Utc>,
) -> super::RunSummary {
    let clock = FixedClock::new(started_at + chrono::TimeDelta::minutes(30));
    let events = NoopEventSink;
    PraxisRuntime {
        config,
        paths,
        backend: &StubBackend,
        clock: &clock,
        events: &events,
        goal_parser: &MarkdownGoalParser,
        identity,
        store,
        tools: &FileToolRegistry,
    }
    .run_once(RunOptions {
        once: true,
        force: false,
        task: None,
    })
    .unwrap()
}
