use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepeatedWorkPattern {
    pub label: String,
    pub sessions: usize,
    pub distinct_days: usize,
    pub latest_outcome: String,
}

#[derive(Debug)]
pub(super) struct SessionRow {
    pub day: i64,
    pub outcome: String,
    pub reviewer_failures: i64,
    pub eval_failures: i64,
    pub repeated_reads_avoided: i64,
    pub selected_goal_id: Option<String>,
    pub selected_goal_title: Option<String>,
    pub selected_task: Option<String>,
}

pub(super) fn recent_sessions(connection: &Connection, limit: usize) -> Result<Vec<SessionRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT
                day, outcome, reviewer_failures, eval_failures, repeated_reads_avoided,
                selected_goal_id, selected_goal_title, selected_task
            FROM sessions
            ORDER BY id DESC
            LIMIT ?1
            ",
        )
        .context("failed to prepare Argus session query")?;
    let rows = statement
        .query_map(params![limit as i64], |row| {
            Ok(SessionRow {
                day: row.get(0)?,
                outcome: row.get(1)?,
                reviewer_failures: row.get(2)?,
                eval_failures: row.get(3)?,
                repeated_reads_avoided: row.get(4)?,
                selected_goal_id: row.get(5)?,
                selected_goal_title: row.get(6)?,
                selected_task: row.get(7)?,
            })
        })
        .context("failed to execute Argus session query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load recent sessions for Argus")
}

pub(super) fn cluster_failures(sessions: &[SessionRow]) -> Vec<(String, usize)> {
    let mut clusters = BTreeMap::new();
    for session in sessions {
        if matches!(
            session.outcome.as_str(),
            "goal_selected" | "task_selected" | "tool_executed" | "stop_condition_met"
        ) {
            continue;
        }
        *clusters.entry(session.outcome.clone()).or_insert(0) += 1;
    }

    let mut pairs = clusters.into_iter().collect::<Vec<_>>();
    pairs.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    pairs
}

pub(super) fn repeated_work_patterns(sessions: &[SessionRow]) -> Vec<RepeatedWorkPattern> {
    let mut clusters: BTreeMap<String, (usize, BTreeSet<i64>, String)> = BTreeMap::new();
    for session in sessions {
        let Some(label) = work_label(session) else {
            continue;
        };
        let entry = clusters
            .entry(label)
            .or_insert_with(|| (0, BTreeSet::new(), session.outcome.clone()));
        entry.0 += 1;
        entry.1.insert(session.day);
    }

    let mut patterns = clusters
        .into_iter()
        .filter_map(|(label, (sessions, days, latest_outcome))| {
            (sessions >= 2).then_some(RepeatedWorkPattern {
                label,
                sessions,
                distinct_days: days.len(),
                latest_outcome,
            })
        })
        .collect::<Vec<_>>();
    patterns.sort_by(|left, right| {
        right
            .distinct_days
            .cmp(&left.distinct_days)
            .then(right.sessions.cmp(&left.sessions))
            .then(left.label.cmp(&right.label))
    });
    patterns
}

fn work_label(session: &SessionRow) -> Option<String> {
    session
        .selected_task
        .as_ref()
        .map(|task| format!("task: {task}"))
        .or_else(|| {
            session
                .selected_goal_id
                .as_ref()
                .zip(session.selected_goal_title.as_ref())
                .map(|(id, title)| format!("goal: {id} {title}"))
        })
}
