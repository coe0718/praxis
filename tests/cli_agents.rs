mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn agents_notes_are_seeded_and_appendable() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let seeded = fs::read_to_string(data_dir.join("AGENTS.md")).unwrap();
    assert!(seeded.contains("## Workflow Notes"));
    assert!(seeded.contains("## Gotchas"));
    assert!(seeded.contains("## Handoffs"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("agents")
        .arg("add")
        .arg("--section")
        .arg("workflow")
        .arg("--note")
        .arg("Prefer project-local scripts over one-off shell commands.")
        .assert()
        .success()
        .stdout(predicate::str::contains("agents: updated"))
        .stdout(predicate::str::contains("section: Workflow Notes"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("agents")
        .arg("add")
        .arg("--section")
        .arg("gotcha")
        .arg("--note")
        .arg("Docker rebuilds are expected to take a while after backend changes.")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("agents")
        .arg("view")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Prefer project-local scripts over one-off shell commands.",
        ))
        .stdout(predicate::str::contains(
            "Docker rebuilds are expected to take a while after backend changes.",
        ));
}
