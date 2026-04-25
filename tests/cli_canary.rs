mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn freeze_blocks_remote_backend_until_a_canary_passes() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command().arg("--data-dir").arg(&data_dir).arg("init").assert().success();

    let config_path = data_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace("backend = \"stub\"", "backend = \"claude\"")
        .replace("freeze_on_model_regression = false", "freeze_on_model_regression = true");
    fs::write(&config_path, updated).unwrap();

    praxis_command()
        .env("PRAXIS_CLAUDE_STUB_RESPONSE", "blocked answer")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("canary")
        .arg("gate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("passing canary"));

    praxis_command()
        .env("PRAXIS_CLAUDE_STUB_RESPONSE", "PraxisCanaryReady")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("canary")
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("claude"))
        .stdout(predicate::str::contains("passed"));

    praxis_command()
        .env("PRAXIS_CLAUDE_STUB_RESPONSE", "allowed answer")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("canary")
        .arg("gate")
        .assert()
        .success()
        .stdout(predicate::str::contains("answer: allowed answer"));
}
