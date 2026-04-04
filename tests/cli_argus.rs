mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn argus_surfaces_recent_quality_directives() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-03T12:00:00Z")
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
        .env("PRAXIS_FIXED_NOW", "2026-04-03T12:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: review_failed"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("argus")
        .assert()
        .success()
        .stdout(predicate::str::contains("review_failures: 1"))
        .stdout(predicate::str::contains("failure_clusters:"))
        .stdout(predicate::str::contains("Tighten completion discipline"));
}
