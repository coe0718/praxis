mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn learning_notes_append_structured_operational_entries() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T08:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T09:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("note")
        .arg("Prefer")
        .arg("cargo")
        .arg("test")
        .arg("--locked")
        .arg("before")
        .arg("pushing.")
        .assert()
        .success()
        .stdout(predicate::str::contains("learning: noted"))
        .stdout(predicate::str::contains("kind: operational-note"))
        .stdout(predicate::str::contains(
            "summary: Prefer cargo test --locked before pushing.",
        ));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T10:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("note")
        .arg("Keep")
        .arg("tool")
        .arg("descriptions")
        .arg("short")
        .arg("and")
        .arg("specific.")
        .assert()
        .success();

    let learnings = fs::read_to_string(data_dir.join("LEARNINGS.md")).unwrap();
    assert!(learnings.contains("# Learnings"));
    assert!(learnings.contains("## 2026-04-05T09:00:00+00:00"));
    assert!(learnings.contains("## 2026-04-05T10:00:00+00:00"));
    assert_eq!(learnings.matches("- Kind: operational-note").count(), 2);
    assert!(learnings.contains("- Source: cli/manual"));
    assert!(learnings.contains("Prefer cargo test --locked before pushing."));
    assert!(learnings.contains("Keep tool descriptions short and specific."));
}
