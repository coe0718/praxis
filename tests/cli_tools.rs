mod common;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn tool_requests_flow_through_queue_and_execute_after_approval() {
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
        .assert()
        .success()
        .stdout(predicate::str::contains("request: pending"))
        .stdout(predicate::str::contains("id: 1"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("queue")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 pending praxis-data-write"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("approve")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("approval: approved"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: tool_executed"))
        .stdout(predicate::str::contains("task: praxis-data-write"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("queue")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("#1 executed praxis-data-write"));
}

#[test]
fn tool_request_rejects_locked_paths() {
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
        .arg("Touch config")
        .arg("--write-path")
        .arg("praxis.toml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("locked path"));
}
