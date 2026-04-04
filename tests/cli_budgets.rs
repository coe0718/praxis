mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn ask_respects_the_budget_policy_before_calling_a_provider() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-08T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("budgets.toml"),
        "[run]\nmax_attempts = 6\nmax_tokens = 3000\nmax_cost_usd = 0.25\n\n[ask]\nmax_attempts = 1\nmax_tokens = 20\nmax_cost_usd = 0.05\n",
    )
    .unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("please")
        .arg("summarize")
        .arg("this")
        .arg("stateful")
        .arg("runtime")
        .assert()
        .success()
        .stdout(predicate::str::contains("mode: ask"))
        .stdout(predicate::str::contains("ask budget blocked"));
}

#[test]
fn run_stops_after_the_budget_is_exhausted() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-08T13:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("budgets.toml"),
        "[run]\nmax_attempts = 1\nmax_tokens = 3000\nmax_cost_usd = 0.25\n\n[ask]\nmax_attempts = 1\nmax_tokens = 600\nmax_cost_usd = 0.05\n",
    )
    .unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-08T13:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .arg("--task")
        .arg("budgeted runtime check")
        .assert()
        .success()
        .stdout(predicate::str::contains("outcome: budget_exhausted"))
        .stdout(predicate::str::contains("run budget blocked"));
}
