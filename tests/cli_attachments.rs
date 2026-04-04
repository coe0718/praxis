mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn oversized_attachments_reject_by_default() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");
    let attachment = temp.path().join("large.txt");

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(&attachment, "a".repeat(100_500)).unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("--file")
        .arg(&attachment)
        .arg("review")
        .arg("this")
        .assert()
        .failure()
        .stderr(predicate::str::contains("exceeds 100000 bytes"));
}

#[test]
fn oversized_attachments_can_be_chunked_or_summarized() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");
    let attachment = temp.path().join("large.txt");

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    fs::write(&attachment, "Heading\n".to_string() + &"x".repeat(100_500)).unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("--attachment-policy")
        .arg("chunk")
        .arg("--file")
        .arg(&attachment)
        .arg("review")
        .arg("this")
        .assert()
        .success()
        .stdout(predicate::str::contains("attachments: 1"))
        .stdout(predicate::str::contains("chunked into"));

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("--attachment-policy")
        .arg("summarize")
        .arg("--file")
        .arg(&attachment)
        .arg("review")
        .arg("this")
        .assert()
        .success()
        .stdout(predicate::str::contains("summarized instead of truncating"));
}
