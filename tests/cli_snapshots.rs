mod common;

use std::{fs, path::PathBuf};

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn enabled_daily_snapshots_create_and_prune_backup_bundles() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let config_path = data_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace(
            "daily_backup_snapshots = false",
            "daily_backup_snapshots = true",
        )
        .replace("snapshot_retention_days = 7", "snapshot_retention_days = 1");
    fs::write(&config_path, updated).unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot: created"));

    let first_snapshot = data_dir.join("backups").join("snapshot-20260404");
    assert!(first_snapshot.join("manifest.json").exists());

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T12:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("snapshot: created"));

    let backups = fs::read_dir(data_dir.join("backups"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>();
    assert_eq!(backups.len(), 1);
    assert!(backups[0].ends_with("snapshot-20260405"));
}
