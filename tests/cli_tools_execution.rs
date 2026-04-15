mod common;

use std::fs;

use praxis::{
    paths::PraxisPaths,
    storage::{ApprovalStatus, StoredApprovalRequest},
    tools::{ToolKind, ToolManifest, execute_request},
};
use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[cfg(unix)]
use std::os::unix::fs::symlink;

fn write_manifest() -> ToolManifest {
    ToolManifest {
        name: "praxis-data-write".to_string(),
        description: "append notes".to_string(),
        kind: ToolKind::Shell,
        required_level: 2,
        requires_approval: true,
        rehearsal_required: true,
        allowed_paths: vec!["JOURNAL.md".to_string(), "PROPOSALS.md".to_string()],
        path: None,
        args: Vec::new(),
        timeout_secs: None,
    }
}

fn write_request(paths: Vec<&str>) -> StoredApprovalRequest {
    StoredApprovalRequest {
        id: 1,
        tool_name: "praxis-data-write".to_string(),
        summary: "append note".to_string(),
        requested_by: "operator".to_string(),
        write_paths: paths.into_iter().map(ToString::to_string).collect(),
        payload_json: Some("{\"append_text\":\"Approved operator note\"}".to_string()),
        status: ApprovalStatus::Approved,
        status_note: None,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn unsupported_approved_tools_fall_back_to_safe_stub_execution() {
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
        .arg("tools")
        .arg("register")
        .arg("--name")
        .arg("custom-review-tool")
        .arg("--description")
        .arg("Custom tool without runtime adapter")
        .arg("--kind")
        .arg("shell")
        .arg("--level")
        .arg("2")
        .arg("--approval")
        .arg("--allow-path")
        .arg("JOURNAL.md")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("tools")
        .arg("request")
        .arg("--name")
        .arg("custom-review-tool")
        .arg("--summary")
        .arg("Run custom review")
        .arg("--write-path")
        .arg("JOURNAL.md")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("approve")
        .arg("1")
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
        .stdout(predicate::str::contains("outcome: tool_executed"))
        .stdout(predicate::str::contains(
            "No execution adapter is installed",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("queue")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 executed custom-review-tool"));
}

#[test]
fn legacy_data_write_approvals_with_null_payload_do_not_deadlock_queue() {
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
        .arg("tools")
        .arg("request")
        .arg("--name")
        .arg("praxis-data-write")
        .arg("--summary")
        .arg("Update journal entry")
        .arg("--write-path")
        .arg("JOURNAL.md")
        .arg("--append-text")
        .arg("Approved operator note")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("approve")
        .arg("1")
        .assert()
        .success();

    let connection = rusqlite::Connection::open(data_dir.join("praxis.db")).unwrap();
    connection
        .execute(
            "UPDATE approval_requests SET payload_json = NULL WHERE id = 1",
            [],
        )
        .unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: tool_executed"))
        .stdout(predicate::str::contains(
            "Legacy approved request had no structured payload",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("queue")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 executed praxis-data-write"));

    let journal = fs::read_to_string(data_dir.join("JOURNAL.md")).unwrap();
    assert!(!journal.contains("Approved operator note"));
}

#[test]
fn retries_skip_files_that_already_contain_the_append_block() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    fs::create_dir_all(&paths.data_dir).unwrap();
    fs::write(&paths.journal_file, "Approved operator note\n").unwrap();

    execute_request(
        &paths,
        &write_manifest(),
        &write_request(vec!["JOURNAL.md", "PROPOSALS.md"]),
    )
    .unwrap();

    let journal = fs::read_to_string(&paths.journal_file).unwrap();
    let proposals = fs::read_to_string(&paths.proposals_file).unwrap();

    assert_eq!(journal.matches("Approved operator note").count(), 1);
    assert_eq!(proposals.matches("Approved operator note").count(), 1);
}

#[cfg(unix)]
#[test]
fn rejects_symlink_targets_before_any_append_happens() {
    let temp = tempdir().unwrap();
    let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
    fs::create_dir_all(&paths.data_dir).unwrap();
    fs::write(&paths.journal_file, "Safe journal\n").unwrap();
    let outside = temp.path().join("outside.txt");
    fs::write(&outside, "outside\n").unwrap();
    symlink(&outside, &paths.proposals_file).unwrap();

    let error = execute_request(
        &paths,
        &write_manifest(),
        &write_request(vec!["JOURNAL.md", "PROPOSALS.md"]),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("symlink target"));
    let journal = fs::read_to_string(&paths.journal_file).unwrap();
    let outside = fs::read_to_string(&outside).unwrap();
    assert!(!journal.contains("Approved operator note"));
    assert_eq!(outside, "outside\n");
}
