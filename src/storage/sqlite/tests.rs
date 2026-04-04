use chrono::TimeZone;
use tempfile::tempdir;

use crate::{
    memory::{MemoryStore, NewColdMemory, NewHotMemory},
    storage::{
        ApprovalStatus, ApprovalStore, EvalRunRecord, EvalSeverity, EvalStatus, NewApprovalRequest,
        ProviderUsageStore, QualityStore, ReviewRecord, ReviewStatus, SessionQualityUpdate,
        SessionRecord, SessionStore,
    },
    usage::ProviderAttempt,
};

use super::SqliteSessionStore;

#[test]
fn initializes_and_records_sessions() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();
    store.validate_schema().unwrap();

    let record = SessionRecord {
        day: 0,
        started_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
        ended_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 5, 0).unwrap(),
        outcome: "goal_selected".to_string(),
        selected_goal_id: Some("G-001".to_string()),
        selected_goal_title: Some("Ship foundation".to_string()),
        selected_task: None,
        action_summary: "Stub backend prepared the goal.".to_string(),
        phase_durations_json: "{\"orient\":0}".to_string(),
        repeated_reads_avoided: 0,
    };

    let stored = store.record_session(&record).unwrap();
    assert_eq!(stored.session_num, 1);
    assert_eq!(
        store.last_session().unwrap().unwrap().outcome,
        "goal_selected"
    );
}

#[test]
fn stores_and_searches_hot_and_cold_memories() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    store
        .insert_hot_memory(NewHotMemory {
            content: "Operator prefers Rust-first local tooling".to_string(),
            summary: Some("Rust-first tooling".to_string()),
            importance: 0.8,
            tags: vec!["operator".to_string()],
            expires_at: None,
        })
        .unwrap();
    store
        .insert_cold_memory(NewColdMemory {
            content: "Praxis prioritizes local-first autonomy and privacy".to_string(),
            weight: 1.2,
            tags: vec!["identity".to_string()],
            source_ids: vec![1],
            contradicts: vec![],
        })
        .unwrap();

    let recent_hot = store.recent_hot_memories(5).unwrap();
    let cold = store.strongest_cold_memories(5).unwrap();
    let search = store.search_memories("local autonomy", 5).unwrap();

    assert_eq!(recent_hot.len(), 1);
    assert_eq!(cold.len(), 1);
    assert!(!search.is_empty());
}

#[test]
fn queues_and_updates_approval_requests() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    let queued = store
        .queue_approval(&NewApprovalRequest {
            tool_name: "praxis-data-write".to_string(),
            summary: "Update JOURNAL.md".to_string(),
            requested_by: "operator".to_string(),
            write_paths: vec!["JOURNAL.md".to_string()],
            status: ApprovalStatus::Pending,
        })
        .unwrap();

    assert_eq!(queued.status, ApprovalStatus::Pending);
    assert_eq!(store.list_approvals(None).unwrap().len(), 1);

    let approved = store
        .set_approval_status(queued.id, ApprovalStatus::Approved, Some("looks good"))
        .unwrap()
        .unwrap();
    assert_eq!(approved.status, ApprovalStatus::Approved);
    assert_eq!(
        store.next_approved_request().unwrap().unwrap().tool_name,
        "praxis-data-write"
    );

    store.mark_approval_consumed(queued.id).unwrap();
    let executed = store.get_approval(queued.id).unwrap().unwrap();
    assert_eq!(executed.status, ApprovalStatus::Executed);
}

#[test]
fn records_review_and_eval_quality_metadata() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    let session = store
        .record_session(&SessionRecord {
            day: 0,
            started_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            ended_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 5, 0).unwrap(),
            outcome: "goal_selected".to_string(),
            selected_goal_id: Some("G-001".to_string()),
            selected_goal_title: Some("Ship foundation".to_string()),
            selected_task: None,
            action_summary: "Stub backend prepared the goal.".to_string(),
            phase_durations_json: "{\"orient\":0}".to_string(),
            repeated_reads_avoided: 0,
        })
        .unwrap();

    store
        .record_review(&ReviewRecord {
            session_id: session.id,
            goal_id: Some("G-001".to_string()),
            status: ReviewStatus::Passed,
            summary: "Reviewer passed G-001.".to_string(),
            findings_json: "[]".to_string(),
            reviewed_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 6, 0).unwrap(),
        })
        .unwrap();
    store
        .record_eval_run(&EvalRunRecord {
            session_id: session.id,
            eval_id: "foundation-smoke".to_string(),
            eval_name: "Foundation smoke".to_string(),
            status: EvalStatus::Passed,
            severity: EvalSeverity::Functional,
            summary: "Eval passed.".to_string(),
            evaluated_at: chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 7, 0).unwrap(),
        })
        .unwrap();
    store
        .update_session_quality(
            session.id,
            &SessionQualityUpdate {
                outcome: "goal_selected".to_string(),
                action_summary: "Updated summary.".to_string(),
                reviewer_passes: 1,
                reviewer_failures: 0,
                eval_passes: 1,
                eval_failures: 0,
            },
        )
        .unwrap();

    assert_eq!(
        store.last_review().unwrap().unwrap().status,
        ReviewStatus::Passed
    );
    assert_eq!(store.latest_eval_summary().unwrap().unwrap().passed, 1);
}

#[test]
fn records_latest_provider_usage() {
    let temp = tempdir().unwrap();
    let store = SqliteSessionStore::new(temp.path().join("praxis.db"));
    store.initialize().unwrap();

    let session = store
        .record_session(&SessionRecord {
            day: 0,
            started_at: chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            ended_at: chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 5, 0).unwrap(),
            outcome: "goal_selected".to_string(),
            selected_goal_id: Some("G-002".to_string()),
            selected_goal_title: Some("Use provider fallback".to_string()),
            selected_task: None,
            action_summary: "Provider routing completed.".to_string(),
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
                    provider: "claude".to_string(),
                    model: "claude-3-5-sonnet-latest".to_string(),
                    success: false,
                    input_tokens: 0,
                    output_tokens: 0,
                    estimated_cost_micros: 0,
                    error: Some("simulated rate limit".to_string()),
                },
                ProviderAttempt {
                    phase: "decide".to_string(),
                    provider: "openai".to_string(),
                    model: "gpt-5.4-mini".to_string(),
                    success: true,
                    input_tokens: 120,
                    output_tokens: 45,
                    estimated_cost_micros: 321,
                    error: None,
                },
            ],
        )
        .unwrap();

    let usage = store.latest_provider_usage().unwrap().unwrap();
    assert_eq!(usage.session_id, session.id);
    assert_eq!(usage.attempt_count, 2);
    assert_eq!(usage.failure_count, 1);
    assert_eq!(usage.last_provider, "openai");
    assert_eq!(usage.tokens_used, 165);
    assert_eq!(usage.estimated_cost_micros, 321);
}
