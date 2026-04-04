mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn run_once_records_review_and_eval_status() {
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
        .stdout(predicate::str::contains("outcome: goal_selected"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("last_review: passed"))
        .stdout(predicate::str::contains(
            "last_eval: passed=1 failed=0 skipped=0 trust_failures=0",
        ));
}

#[test]
fn run_once_surfaces_reviewer_failures() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
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
  "commands": ["false"]
}
"#,
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
        .stdout(predicate::str::contains("outcome: review_failed"));

    let journal = fs::read_to_string(data_dir.join("JOURNAL.md")).unwrap();
    assert!(journal.contains("Outcome: review_failed"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("last_review: failed"))
        .stdout(predicate::str::contains("operational_memory: dnr=1 bugs=1"));
}

#[test]
fn run_once_surfaces_eval_failures() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-03-31T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("evals/foundation-smoke.json"),
        r#"{
  "id": "foundation-smoke",
  "name": "Foundation runtime files stay healthy",
  "when": "always",
  "severity": "functional",
  "scenario": "A failing eval should mark the session as regressed.",
  "expected_behavior": ["Eval commands succeed"],
  "forbidden_behavior": ["Eval commands fail"],
  "verify_with": "shell",
  "commands": ["false"]
}
"#,
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
        .stdout(predicate::str::contains("outcome: eval_failed"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "last_eval: passed=0 failed=1 skipped=0 trust_failures=0",
        ))
        .stdout(predicate::str::contains("operational_memory: dnr=1 bugs=1"));
}
