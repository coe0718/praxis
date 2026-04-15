mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn boundaries_are_editable_and_track_weekly_review_state() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("boundaries")
        .arg("show")
        .assert()
        .success()
        .stdout(predicate::str::contains("boundary_review_due: true"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("boundaries")
        .arg("add")
        .arg("Never")
        .arg("deploy")
        .arg("after")
        .arg("midnight")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:10:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("boundaries")
        .arg("confirm")
        .arg("--note")
        .arg("Reviewed after setup")
        .assert()
        .success()
        .stdout(predicate::str::contains("boundary_review_confirmed"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("boundary_review_due: false"))
        .stdout(predicate::str::contains(
            "last_boundary_review: 2026-04-04T12:10:00+00:00",
        ));

    let identity = fs::read_to_string(data_dir.join("IDENTITY.md")).unwrap();
    assert!(identity.contains("Never deploy after midnight"));
}
