mod common;

use std::fs;

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
        .arg("--append-text")
        .arg("Approved operator note")
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
        .stdout(predicate::str::contains("task: praxis-data-write"))
        .stdout(predicate::str::contains("appended operator-approved text"));

    let journal = fs::read_to_string(data_dir.join("JOURNAL.md")).unwrap();
    assert!(journal.contains("Approved operator note"));

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
        .arg("--append-text")
        .arg("Nope")
        .assert()
        .failure()
        .stderr(predicate::str::contains("locked path"));
}

#[test]
fn data_write_requests_require_append_text() {
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
        .failure()
        .stderr(predicate::str::contains("--append-text"));
}

#[test]
fn tool_registry_validates_duplicate_on_runtime() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    // Register a custom tool
    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("tools")
        .arg("register")
        .arg("--name")
        .arg("my-tool")
        .arg("--description")
        .arg("A custom test tool")
        .arg("--kind")
        .arg("internal")
        .arg("--level")
        .arg("1")
        .assert()
        .success();

    // Write another file with same tool name to trigger duplicate error
    let tool_path = data_dir.join("tools").join("my-tool-copy.toml");
    fs::write(
        &tool_path,
        r#"
name = "my-tool"
description = "Duplicate tool"
kind = "internal"
required_level = 1
"#,
    )
    .unwrap();

    // Doctor command validates tools and should fail on duplicate
    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate"));
}

#[test]
fn tool_registry_rejects_invalid_manifest() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    // Write an invalid manifest directly
    let tool_path = data_dir.join("tools").join("bad-tool.toml");
    fs::write(&tool_path, "name = ''\ndescription = ''\nkind = 'internal'\nrequired_level = 5")
        .unwrap();

    // Doctor command validates tools and should fail on invalid manifest
    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("tool name must not be empty"));
}
