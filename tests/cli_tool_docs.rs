mod common;

use std::fs;

use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn capabilities_file_tracks_tool_examples_and_failure_notes() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-06T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let seeded = fs::read_to_string(data_dir.join("CAPABILITIES.md")).unwrap();
    assert!(seeded.contains("### praxis-data-write"));
    assert!(seeded.contains("Reliability: no request history yet"));

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

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-06T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
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
        .arg("Attempt risky journal rewrite")
        .arg("--write-path")
        .arg("JOURNAL.md")
        .arg("--append-text")
        .arg("Rejected note")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("reject")
        .arg("2")
        .arg("--note")
        .arg("operator wants a safer path")
        .assert()
        .success();

    let capabilities = fs::read_to_string(data_dir.join("CAPABILITIES.md")).unwrap();
    assert!(capabilities.contains("executed=1"));
    assert!(capabilities.contains("rejected=1"));
    assert!(capabilities.contains("Update journal entry"));
    assert!(capabilities.contains("Attempt risky journal rewrite"));
    assert!(capabilities.contains("operator wants a safer path"));
}
