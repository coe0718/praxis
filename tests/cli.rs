use std::fs;

use assert_cmd::Command;
use praxis::state::{SessionPhase, SessionState};
use predicates::prelude::*;
use tempfile::tempdir;

fn praxis_command() -> Command {
    Command::cargo_bin("praxis").unwrap()
}

#[test]
fn init_is_idempotent_and_creates_foundation_files() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .arg("--name")
        .arg("Praxis Test")
        .arg("--timezone")
        .arg("UTC")
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized: ok"));

    let first_config = fs::read_to_string(data_dir.join("praxis.toml")).unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized: ok"));

    let second_config = fs::read_to_string(data_dir.join("praxis.toml")).unwrap();
    assert_eq!(first_config, second_config);

    for required in [
        "praxis.toml",
        "praxis.db",
        "IDENTITY.md",
        "GOALS.md",
        "ROADMAP.md",
        "JOURNAL.md",
        "METRICS.md",
        "PATTERNS.md",
        "LEARNINGS.md",
        "CAPABILITIES.md",
        "PROPOSALS.md",
        "DAY_COUNT",
    ] {
        assert!(data_dir.join(required).exists(), "{required} should exist");
    }

    assert!(data_dir.join("goals/criteria").exists());
}

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

    let journal = fs::read_to_string(data_dir.join("JOURNAL.md")).unwrap();
    assert!(journal.contains("Goal: G-001: Complete the first Praxis foundation run"));

    let metrics = fs::read_to_string(data_dir.join("METRICS.md")).unwrap();
    assert!(metrics.contains("| 1 | goal_selected |"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
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

#[test]
fn doctor_reports_healthy_and_broken_setups() {
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
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor: ok"));

    fs::remove_file(data_dir.join("GOALS.md")).unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required identity file"));
}
