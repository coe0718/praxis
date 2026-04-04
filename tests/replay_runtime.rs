mod common;
#[path = "common/replay.rs"]
mod replay;

use crate::replay::run_fixture;

#[test]
fn foundation_runtime_replay_stays_stable() {
    run_fixture("foundation-runtime").unwrap();
}

#[test]
fn approval_execution_replay_stays_stable() {
    run_fixture("approval-execution").unwrap();
}
