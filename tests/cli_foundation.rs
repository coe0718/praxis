mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

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
    assert!(data_dir.join("tools").exists());
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
        .stdout(predicate::str::contains("doctor: ok"))
        .stdout(predicate::str::contains("tools: ok"));

    fs::remove_file(data_dir.join("GOALS.md")).unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required identity file"));
}
