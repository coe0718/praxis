use chrono::TimeZone;
use tempfile::tempdir;

use super::{PraxisRuntime, RunOptions, StubBackend};
use crate::{
    config::AppConfig,
    events::NoopEventSink,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    memory::MemoryStore,
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{
        ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionStore, SqliteSessionStore,
    },
    time::FixedClock,
    tools::{FileToolRegistry, LoopGuard, ToolRegistry},
};

#[test]
fn runtime_runs_single_session() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let identity = LocalIdentityPolicy;

    identity
        .ensure_foundation(
            &paths,
            &config,
            chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
        )
        .unwrap();
    FileToolRegistry.ensure_foundation(&paths).unwrap();

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize().unwrap();

    let clock = FixedClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 3, 31, 12, 30, 0)
            .unwrap(),
    );

    let runtime = PraxisRuntime {
        config: &config,
        paths: &paths,
        backend: &StubBackend,
        clock: &clock,
        events: &NoopEventSink,
        goal_parser: &MarkdownGoalParser,
        identity: &identity,
        store: &store,
        tools: &FileToolRegistry,
    };

    let summary = runtime
        .run_once(RunOptions {
            once: true,
            force: false,
            task: None,
        })
        .unwrap();

    assert_eq!(summary.phase, SessionPhase::Sleep);
    assert_eq!(summary.outcome, "goal_selected");
    assert!(store.recent_hot_memories(5).unwrap().len() >= 1);

    let state = SessionState::load(&paths.state_file).unwrap().unwrap();
    assert!(
        state
            .orientation_summary
            .unwrap_or_default()
            .contains("Context used")
    );
}

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
            status: ApprovalStatus::Approved,
        })
        .unwrap();

    let mut state = SessionState {
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
        selected_tool_request_id: Some(approved.id),
        tool_invocation_hashes: Vec::new(),
    };
    let invocation = "praxis-data-write|Update JOURNAL.md|JOURNAL.md";
    LoopGuard.record(&mut state, invocation, 3);
    LoopGuard.record(&mut state, invocation, 3);
    state.save(&paths.state_file).unwrap();

    let clock = FixedClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 3, 31, 12, 30, 0)
            .unwrap(),
    );

    let runtime = PraxisRuntime {
        config: &config,
        paths: &paths,
        backend: &StubBackend,
        clock: &clock,
        events: &NoopEventSink,
        goal_parser: &MarkdownGoalParser,
        identity: &identity,
        store: &store,
        tools: &FileToolRegistry,
    };

    let summary = runtime
        .run_once(RunOptions {
            once: true,
            force: false,
            task: None,
        })
        .unwrap();

    assert_eq!(summary.outcome, "blocked_loop_guard");
    assert_eq!(
        store.get_approval(approved.id).unwrap().unwrap().status,
        ApprovalStatus::Approved
    );
}

#[test]
fn runtime_reaches_stop_condition_when_all_goals_are_done() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let identity = LocalIdentityPolicy;
    let started_at = chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();

    identity
        .ensure_foundation(&paths, &config, started_at)
        .unwrap();
    std::fs::write(&paths.goals_file, "# Goals\n\n- [x] G-001: Done already\n").unwrap();
    FileToolRegistry.ensure_foundation(&paths).unwrap();

    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize().unwrap();
    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 30, 0).unwrap());

    let runtime = PraxisRuntime {
        config: &config,
        paths: &paths,
        backend: &StubBackend,
        clock: &clock,
        events: &NoopEventSink,
        goal_parser: &MarkdownGoalParser,
        identity: &identity,
        store: &store,
        tools: &FileToolRegistry,
    };

    let summary = runtime
        .run_once(RunOptions {
            once: true,
            force: false,
            task: None,
        })
        .unwrap();

    assert_eq!(summary.outcome, "stop_condition_met");
}
