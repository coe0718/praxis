mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn router_falls_back_to_openai_and_reports_provider_usage() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-03T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let config_path = data_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace("backend = \"stub\"", "backend = \"router\"");
    fs::write(&config_path, updated).unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-03T12:30:00Z")
        .env("ANTHROPIC_API_KEY", "test-key")
        .env("PRAXIS_CLAUDE_FORCE_ERROR", "simulated rate limit")
        .env(
            "PRAXIS_OPENAI_STUB_RESPONSE",
            "OpenAI fallback planned the next safe step.",
        )
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: goal_selected"))
        .stdout(predicate::str::contains(
            "summary: OpenAI fallback planned the next safe step.",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("backend: router"))
        .stdout(predicate::str::contains("last_provider: openai"))
        .stdout(predicate::str::contains("attempts=6"))
        .stdout(predicate::str::contains("failures=3"));
}

#[test]
fn local_first_fallback_prefers_ollama_for_low_risk_asks() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-08T14:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let config_path = data_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace("backend = \"stub\"", "backend = \"claude\"")
        .replace(
            "local_first_fallback = false",
            "local_first_fallback = true",
        );
    fs::write(&config_path, updated).unwrap();

    praxis_command()
        .env("PRAXIS_CLAUDE_STUB_RESPONSE", "Claude cloud answer")
        .env(
            "PRAXIS_OLLAMA_STUB_RESPONSE",
            "Ollama local fallback answer",
        )
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("give")
        .arg("me")
        .arg("a")
        .arg("brief")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "answer: Ollama local fallback answer",
        ));
}
