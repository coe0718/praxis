mod common;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn heartbeat_status_and_check_report_recent_runtime_health() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T09:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T09:10:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("heartbeat")
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("heartbeat: ok"))
        .stdout(predicate::str::contains("component: praxis"))
        .stdout(predicate::str::contains("phase: sleep"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T09:12:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("heartbeat")
        .arg("check")
        .arg("--max-age-seconds")
        .arg("600")
        .assert()
        .success()
        .stdout(predicate::str::contains("heartbeat_check: ok"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-09T11:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("heartbeat")
        .arg("check")
        .arg("--max-age-seconds")
        .arg("60")
        .assert()
        .failure()
        .stderr(predicate::str::contains("heartbeat is stale"));
}
