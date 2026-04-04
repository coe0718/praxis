mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

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
