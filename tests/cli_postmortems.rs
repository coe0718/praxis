mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn reviewer_failures_generate_postmortems() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-07T09:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("goals/criteria/G-001.json"),
        r#"{
  "goal_id": "G-001",
  "done_when": ["A failing reviewer gate should block completion"],
  "verify_with": "shell",
  "commands": ["exit 1"]
}
"#,
    )
    .unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-07T09:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: review_failed"));

    let postmortems = fs::read_to_string(data_dir.join("POSTMORTEMS.md")).unwrap();
    assert!(postmortems.contains("# Postmortems"));
    assert!(postmortems.contains("Outcome: review_failed"));
    assert!(postmortems.contains("Trigger: review gate failed"));
}

#[test]
fn eval_failures_generate_postmortems() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-07T10:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("evals/foundation-smoke.json"),
        r#"{
  "id": "foundation-smoke",
  "name": "Foundation smoke",
  "when": "always",
  "severity": "functional",
  "scenario": "A failing eval should mark the session as regressed.",
  "expected_behavior": ["A failing shell command trips the eval summary."],
  "forbidden_behavior": ["Praxis reports a healthy eval result."],
  "verify_with": "shell",
  "commands": ["exit 1"]
}
"#,
    )
    .unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-07T10:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: eval_failed"));

    let postmortems = fs::read_to_string(data_dir.join("POSTMORTEMS.md")).unwrap();
    assert!(postmortems.contains("Outcome: eval_failed"));
    assert!(postmortems.contains("Trigger: operator eval failed"));
    assert!(postmortems.contains("Foundation smoke"));
}
