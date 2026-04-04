mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn offline_profile_forces_local_backend_and_surfaces_in_status() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-11T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let config_path = data_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace("profile = \"quality\"", "profile = \"offline\"")
        .replace("backend = \"stub\"", "backend = \"claude\"");
    fs::write(&config_path, updated).unwrap();

    praxis_command()
        .env(
            "PRAXIS_OLLAMA_STUB_RESPONSE",
            "Offline profile local answer",
        )
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("profile")
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "answer: Offline profile local answer",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("profile: offline"))
        .stdout(predicate::str::contains("backend: ollama"));
}
