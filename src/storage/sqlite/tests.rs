use chrono::TimeZone;
use tempfile::tempdir;

use crate::{
    memory::{MemoryStore, NewColdMemory, NewHotMemory},
    storage::{ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionRecord, SessionStore},
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
