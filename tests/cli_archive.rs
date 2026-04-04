mod common;

use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn state_export_and_import_restore_portable_runtime_state() {
    let temp = tempdir().unwrap();
    let source_dir = temp.path().join("source");
    let bundle_dir = temp.path().join("bundle");
    let restored_dir = temp.path().join("restored");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:00:00Z")
        .arg("--data-dir")
        .arg(&source_dir)
        .arg("init")
        .assert()
        .success();

    let absolute_db = source_dir.join("runtime/praxis.db");
    let absolute_state = source_dir.join("runtime/session_state.json");
    let config_path = source_dir.join("praxis.toml");
    let updated = fs::read_to_string(&config_path)
        .unwrap()
        .replace(
            "path = \"praxis.db\"",
            &format!("path = \"{}\"", absolute_db.display()),
        )
        .replace(
            "state_file = \"session_state.json\"",
            &format!("state_file = \"{}\"", absolute_state.display()),
        );
    fs::write(&config_path, updated).unwrap();

    fs::create_dir_all(source_dir.join("runtime")).unwrap();
    fs::rename(source_dir.join("praxis.db"), &absolute_db).unwrap();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:15:00Z")
        .arg("--data-dir")
        .arg(&source_dir)
        .arg("run")
        .arg("--once")
        .arg("--task")
        .arg("portable restore smoke test")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T12:30:00Z")
        .arg("--data-dir")
        .arg(&source_dir)
        .arg("export")
        .arg("state")
        .arg("--output")
        .arg(&bundle_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("export: ok"))
        .stdout(predicate::str::contains("kind: state"));

    assert!(bundle_dir.join("manifest.json").exists());
    assert!(bundle_dir.join("runtime/database.sqlite").exists());

    fs::remove_dir_all(&source_dir).unwrap();

    praxis_command()
        .arg("--data-dir")
        .arg(&restored_dir)
        .arg("import")
        .arg("--input")
        .arg(&bundle_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("import: ok"));

    praxis_command()
        .arg("--data-dir")
        .arg(&restored_dir)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor: ok"));

    let restored_config = fs::read_to_string(restored_dir.join("praxis.toml")).unwrap();
    assert!(restored_config.contains(&restored_dir.display().to_string()));
    assert!(!restored_config.contains(&source_dir.display().to_string()));
    assert!(restored_dir.join("runtime/praxis.db").exists());
    assert!(restored_dir.join("runtime/session_state.json").exists());

    praxis_command()
        .arg("--data-dir")
        .arg(&restored_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("last_session: #1"))
        .stdout(predicate::str::contains(
            "last_session_ended_at: 2026-04-04T12:15:00+00:00",
        ));
}

#[test]
fn audit_export_writes_human_readable_runtime_report() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");
    let audit_file = temp.path().join("audit.md");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T13:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T13:05:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("tools")
        .arg("request")
        .arg("--name")
        .arg("praxis-data-write")
        .arg("--summary")
        .arg("Append audit note")
        .arg("--write-path")
        .arg("JOURNAL.md")
        .arg("--append-text")
        .arg("Audit export exercised a real tool execution.")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("approve")
        .arg("1")
        .assert()
        .success();

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T13:10:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .assert()
        .success()
        .stdout(predicate::str::contains("tool_executed"));

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-04T13:20:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("export")
        .arg("audit")
        .arg("--output")
        .arg(&audit_file)
        .arg("--days")
        .arg("30")
        .assert()
        .success()
        .stdout(predicate::str::contains("kind: audit"));

    let report = fs::read_to_string(&audit_file).unwrap();
    assert!(report.contains("# Praxis Audit Export"));
    assert!(report.contains("## Sessions"));
    assert!(report.contains("tool_executed"));
    assert!(report.contains("## Approvals"));
    assert!(report.contains("praxis-data-write"));
    assert!(report.contains("## Recent Events"));
}
