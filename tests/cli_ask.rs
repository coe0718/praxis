mod common;

use predicates::prelude::*;
use rusqlite::Connection;
use tempfile::tempdir;

use crate::common::praxis_command;

#[test]
fn ask_is_stateless_while_run_records_a_real_session() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T09:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    praxis_command()
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("ask")
        .arg("what")
        .arg("is")
        .arg("the")
        .arg("current")
        .arg("mode")
        .assert()
        .success()
        .stdout(predicate::str::contains("mode: ask"))
        .stdout(predicate::str::contains("stateful: false"))
        .stdout(predicate::str::contains(
            "answer: Stub backend answered without creating a session",
        ));

    assert_eq!(session_count(&data_dir), 0);

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T09:05:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--once")
        .arg("--task")
        .arg("stateful session")
        .assert()
        .success()
        .stdout(predicate::str::contains("task: stateful session"));

    assert_eq!(session_count(&data_dir), 1);
}

#[test]
fn telegram_ask_is_lightweight_but_run_updates_state() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("praxis");

    praxis_command()
        .env("PRAXIS_FIXED_NOW", "2026-04-05T10:00:00Z")
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("init")
        .assert()
        .success();

    let ask_reply =
        praxis::messaging::handle_telegram_command(data_dir.clone(), 42, "/ask summarize the mode")
            .unwrap();
    assert!(ask_reply.contains("mode: ask"));
    assert_eq!(session_count(&data_dir), 0);

    let run_reply =
        praxis::messaging::handle_telegram_command(data_dir.clone(), 42, "/run stateful telegram task")
            .unwrap();
    assert!(run_reply.contains("task: stateful telegram task"));
    assert_eq!(session_count(&data_dir), 1);
}

fn session_count(data_dir: &std::path::Path) -> i64 {
    let connection = Connection::open(data_dir.join("praxis.db")).unwrap();
    connection
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap()
}
