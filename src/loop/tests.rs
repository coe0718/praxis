use chrono::TimeZone;
use tempfile::tempdir;

use super::{PraxisRuntime, RunOptions, StubBackend};
use crate::{
    config::AppConfig,
    events::NoopEventSink,
    identity::{IdentityPolicy, LocalIdentityPolicy, MarkdownGoalParser},
    lite::LiteMode,
    memory::MemoryStore,
    paths::PraxisPaths,
    state::{SessionPhase, SessionState},
    storage::{SessionStore, SqliteSessionStore},
    time::FixedClock,
    tools::{FileToolRegistry, ToolRegistry},
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

    let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 30, 0).unwrap());

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
        lite: &LiteMode::default(),
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
    assert!(!store.recent_hot_memories(5).unwrap().is_empty());

    let state = SessionState::load(&paths.state_file).unwrap().unwrap();
    assert!(state.orientation_summary.unwrap_or_default().contains("Context used"));
}

#[test]
fn runtime_reaches_stop_condition_when_all_goals_are_done() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
    let identity = LocalIdentityPolicy;
    let started_at = chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();

    identity.ensure_foundation(&paths, &config, started_at).unwrap();
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
        lite: &LiteMode::default(),
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
