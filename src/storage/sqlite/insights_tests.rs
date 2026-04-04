use chrono::TimeZone;
use tempfile::tempdir;

use crate::{
    anatomy::NewAnatomyEntry,
    memory::{NewDoNotRepeat, NewKnownBug},
    storage::{
        AnatomyStore, OperationalMemoryStore, ProviderUsageStore, SessionRecord, SessionStore,
    },
    usage::ProviderAttempt,
};

use super::SqliteSessionStore;

#[test]
fn upserts_anatomy_entries_and_counts_them() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    store
        .upsert_anatomy_entry(&NewAnatomyEntry {
            path: "src/main.rs".to_string(),
            description: "CLI entrypoint".to_string(),
            token_estimate: 42,
            last_modified_at: "2026-04-03T12:00:00Z".to_string(),
            tags: vec!["rust".to_string()],
        })
        .unwrap();
    store
        .upsert_anatomy_entry(&NewAnatomyEntry {
            path: "src/main.rs".to_string(),
            description: "Thin binary entrypoint".to_string(),
            token_estimate: 40,
            last_modified_at: "2026-04-03T12:30:00Z".to_string(),
            tags: vec!["rust".to_string()],
        })
        .unwrap();

    assert_eq!(store.anatomy_entry_count().unwrap(), 1);
}

#[test]
fn records_and_searches_operational_memory() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    store
        .record_do_not_repeat(NewDoNotRepeat {
            statement: "Do not reread the whole repo for one symbol.".to_string(),
            tags: vec!["reads".to_string()],
            severity: "review_failed".to_string(),
            source_session_id: Some(1),
            expires_at: None,
        })
        .unwrap();
    store
        .record_known_bug(NewKnownBug {
            signature: "symbol lookup".to_string(),
            symptoms: "Opened too many files before finding the target.".to_string(),
            fix_summary: "Use anatomy summaries first, then narrow the read.".to_string(),
            tags: vec!["anatomy".to_string()],
            source_session_id: Some(1),
        })
        .unwrap();

    let counts = store.operational_memory_counts().unwrap();
    assert_eq!(counts.do_not_repeat, 1);
    assert_eq!(counts.known_bugs, 1);
    assert_eq!(store.search_do_not_repeat("repo", 5).unwrap().len(), 1);
    assert_eq!(store.search_known_bugs("anatomy", 5).unwrap().len(), 1);
}

#[test]
fn records_token_ledger_rows_from_provider_attempts() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    let session = store
        .record_session(&SessionRecord {
            day: 0,
            started_at: chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            ended_at: chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 5, 0).unwrap(),
            outcome: "goal_selected".to_string(),
            selected_goal_id: Some("G-010".to_string()),
            selected_goal_title: Some("Measure token costs".to_string()),
            selected_task: None,
            action_summary: "Recorded token usage.".to_string(),
            phase_durations_json: "{\"orient\":0}".to_string(),
            repeated_reads_avoided: 0,
        })
        .unwrap();

    store
        .record_provider_attempts(
            session.id,
            &[
                ProviderAttempt {
                    phase: "decide".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-5.4-mini".to_string(),
                    success: true,
                    input_tokens: 100,
                    output_tokens: 25,
                    estimated_cost_micros: 200,
                    error: None,
                },
                ProviderAttempt {
                    phase: "act".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-5.4-mini".to_string(),
                    success: true,
                    input_tokens: 80,
                    output_tokens: 20,
                    estimated_cost_micros: 150,
                    error: None,
                },
            ],
        )
        .unwrap();

    let summary = store.latest_token_summary().unwrap().unwrap();
    assert_eq!(summary.tokens_used, 225);
    assert_eq!(summary.estimated_cost_micros, 350);
    let hotspots = store.latest_phase_token_usage(2).unwrap();
    assert_eq!(hotspots[0].phase, "decide");
}
