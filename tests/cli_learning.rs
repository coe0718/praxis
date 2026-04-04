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

    let proposals = fs::read_to_string(data_dir.join("PROPOSALS.md")).unwrap();
    assert!(proposals.contains("## Pending"));
    assert!(proposals.contains("Automate recurring work: task: clean notes"));
    assert!(proposals.contains("Automate recurring work: task: review backlog"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T10:15:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("accept")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("status: accepted"))
        .stdout(predicate::str::contains("promoted_goal: G-002"))
        .stdout(predicate::str::contains("created_goal: true"));

    let goals = fs::read_to_string(data_dir.join("GOALS.md")).unwrap();
    assert!(goals.contains("- [ ] G-002: Automate recurring work: task: clean notes"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T10:20:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("dismiss")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("status: dismissed"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("learn")
        .arg("list")
        .arg("--all")
        .assert()
        .success()
        .stdout(predicate::str::contains("pending_opportunities: 0"))
        .stdout(predicate::str::contains("accepted_opportunities: 1"))
        .stdout(predicate::str::contains("dismissed_opportunities: 1"))
        .stdout(predicate::str::contains("accepted:"))
        .stdout(predicate::str::contains("dismissed:"))
        .stdout(predicate::str::contains(
            "Automate recurring work: task: clean notes -> G-002",
        ));

    fs::write(
        data_dir.join("GOALS.md"),
        fs::read_to_string(data_dir.join("GOALS.md"))
            .unwrap()
            .replace(
                "- [ ] G-001: Complete the first Praxis foundation run",
                "- [x] G-001: Complete the first Praxis foundation run",
            ),
    )
    .unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T10:30:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "goal: G-002: Automate recurring work: task: clean notes",
        ));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("pending_opportunities: 0"))
        .stdout(predicate::str::contains(
            "last_learning: processed=1 changed=1 opportunities=2",
        ))
        .stdout(predicate::str::contains("drift_status: insufficient_data"));

    let proposals = fs::read_to_string(data_dir.join("PROPOSALS.md")).unwrap();
    assert!(proposals.contains("## Accepted"));
    assert!(proposals.contains("## Dismissed"));
    assert!(proposals.contains("status: accepted"));
    assert!(proposals.contains("status: dismissed"));
    assert!(proposals.contains("linked_goal: G-002"));
}
