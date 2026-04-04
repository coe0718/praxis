mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn successful_runs_persist_adaptive_context_feedback() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T12:15:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .arg("--task")
        .arg("adaptive context smoke test")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: task_selected"));

    let adaptation = fs::read_to_string(data_dir.join("context_adaptation.json")).unwrap();
    assert!(adaptation.contains("\"task\""));
    assert!(adaptation.contains("\"successful_sessions\": 1"));
}
