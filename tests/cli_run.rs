mod common;

use praxis::state::{SessionPhase, SessionState};
use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn run_once_records_goal_session_and_updates_status() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: goal_selected"))
        .stdout(predicate::str::contains("phase: sleep"))
        .stdout(predicate::str::contains("resumed: false"));

    let journal = std::fs::read_to_string(data_dir.join("JOURNAL.md")).unwrap();
    assert!(journal.contains("Goal: G-001: Complete the first Praxis foundation run"));

    let metrics = std::fs::read_to_string(data_dir.join("METRICS.md")).unwrap();
    assert!(metrics.contains("| 1 | goal_selected |"));
    let events = std::fs::read_to_string(data_dir.join("events.jsonl")).unwrap();
    assert!(events.contains("agent:orient_start"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("backend: stub"))
        .stdout(predicate::str::contains("registered_tools: 2"))
        .stdout(predicate::str::contains("event_count:"))
        .stdout(predicate::str::contains("phase: sleep"))
        .stdout(predicate::str::contains("last_session: #1 goal_selected"));
}

#[test]
fn run_once_defers_during_quiet_hours() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T23:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: deferred_quiet_hours"))
        .stdout(predicate::str::contains("phase: sleep"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("last_session: none"))
        .stdout(predicate::str::contains(
            "last_outcome: deferred_quiet_hours",
        ));
}

#[test]
fn run_once_resumes_from_interrupted_state() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let state = SessionState {
        current_phase: SessionPhase::Act,
        started_at: chrono::DateTime::parse_from_rfc3339("2026-03-31T11:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-03-31T11:10:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        completed_at: None,
        selected_goal_id: Some("G-777".to_string()),
        selected_goal_title: Some("Resume the interrupted session".to_string()),
        requested_task: None,
        orientation_summary: Some("Loaded prior state.".to_string()),
        action_summary: Some("Resume action.".to_string()),
        last_outcome: Some("goal_selected".to_string()),
        resume_count: 0,
        selected_tool_name: None,
        selected_tool_request_id: None,
        tool_invocation_hashes: Vec::new(),
    };
    state.save(&data_dir.join("session_state.json")).unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("resumed: true"))
        .stdout(predicate::str::contains("outcome: goal_selected"));
}
