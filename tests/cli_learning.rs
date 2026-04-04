mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn learning_runtime_ingests_sources_and_throttles_opportunities() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-03T08:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(
        data_dir.join("learning/sources/operator-notes.md"),
        "# Operator Notes\n\n- Keep recurring briefs concise.\n- Prefer automation for repeated inbox triage.\n",
    )
    .unwrap();

    for task in ["triage inbox", "review backlog", "clean notes"] {
        praxis_command()
            .env("PRAXIS_FIXED_NOW", "2026-04-03T09:00:00Z")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("run")
            .arg("--once")
            .arg("--task")
            .arg(task)
            .assert()
            .success();
    }

    fs::write(data_dir.join("DAY_COUNT"), "1\n").unwrap();

    for task in ["triage inbox", "review backlog", "clean notes"] {
        praxis_command()
            .env("PRAXIS_FIXED_NOW", "2026-04-04T09:00:00Z")
            .arg("--data-dir")
            .arg(&data_dir)
            .arg("run")
            .arg("--once")
            .arg("--task")
            .arg(task)
            .assert()
            .success();
    }

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T10:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("learning: ok"))
        .stdout(predicate::str::contains("sources_processed: 1"))
        .stdout(predicate::str::contains("sources_changed: 1"))
        .stdout(predicate::str::contains("opportunities_created: 2"))
        .stdout(predicate::str::contains("throttle_reached: true"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("pending_opportunities: 2"))
        .stdout(predicate::str::contains(
            "Automate recurring work: task: clean notes",
        ))
        .stdout(predicate::str::contains(
            "Automate recurring work: task: review backlog",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("pending_opportunities: 2"))
        .stdout(predicate::str::contains(
            "last_learning: processed=1 changed=1 opportunities=2",
        ))
        .stdout(predicate::str::contains("drift_status: insufficient_data"));
}
