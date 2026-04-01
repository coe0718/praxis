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
    storage::{SessionStore, SqliteSessionStore},
    time::FixedClock,
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
