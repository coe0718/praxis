use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgusReport {
    pub session_count: usize,
    pub review_failures: i64,
    pub eval_failures: i64,
    pub loop_guard_blocks: i64,
    pub waiting_sessions: usize,
    pub failure_clusters: Vec<(String, usize)>,
    pub directives: Vec<String>,
}

pub fn analyze(database_file: &Path, limit: usize) -> Result<ArgusReport> {
    let connection = Connection::open(database_file)
        .with_context(|| format!("failed to open {}", database_file.display()))?;
    let sessions = recent_sessions(&connection, limit.max(1))?;
    let session_count = sessions.len();
    let review_failures = sessions
        .iter()
        .map(|session| session.reviewer_failures)
        .sum();
    let eval_failures = sessions.iter().map(|session| session.eval_failures).sum();
    let loop_guard_blocks = sessions
        .iter()
        .filter(|session| session.outcome == "blocked_loop_guard")
        .count() as i64;
    let waiting_sessions = sessions
        .iter()
        .filter(|session| session.outcome == "waiting_on_dependencies")
        .count();
    let failure_clusters = cluster_failures(&sessions);
    let directives = directives(
        session_count,
        review_failures,
        eval_failures,
        loop_guard_blocks,
        waiting_sessions,
    );

    Ok(ArgusReport {
        session_count,
        review_failures,
        eval_failures,
        loop_guard_blocks,
        waiting_sessions,
        failure_clusters,
        directives,
    })
}

pub fn render(report: &ArgusReport) -> String {
    let mut lines = vec![
        "argus: ready".to_string(),
        format!("sessions_analyzed: {}", report.session_count),
        format!("review_failures: {}", report.review_failures),
        format!("eval_failures: {}", report.eval_failures),
        format!("loop_guard_blocks: {}", report.loop_guard_blocks),
        format!("waiting_sessions: {}", report.waiting_sessions),
    ];

    if report.failure_clusters.is_empty() {
        lines.push("failure_clusters: none".to_string());
    } else {
        lines.push("failure_clusters:".to_string());
        lines.extend(
            report
                .failure_clusters
                .iter()
                .map(|(name, count)| format!("- {name} x{count}")),
        );
    }

    lines.push("directives:".to_string());
    lines.extend(report.directives.iter().map(|line| format!("- {line}")));
    lines.join("\n")
}

#[derive(Debug)]
struct SessionRow {
    outcome: String,
    reviewer_failures: i64,
    eval_failures: i64,
}

fn recent_sessions(connection: &Connection, limit: usize) -> Result<Vec<SessionRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT outcome, reviewer_failures, eval_failures
            FROM sessions
            ORDER BY id DESC
            LIMIT ?1
            ",
        )
        .context("failed to prepare Argus session query")?;
    let rows = statement
        .query_map(params![limit as i64], |row| {
            Ok(SessionRow {
                outcome: row.get(0)?,
                reviewer_failures: row.get(1)?,
                eval_failures: row.get(2)?,
            })
        })
        .context("failed to execute Argus session query")?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load recent sessions for Argus")
}

fn cluster_failures(sessions: &[SessionRow]) -> Vec<(String, usize)> {
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

fn directives(
    session_count: usize,
    review_failures: i64,
    eval_failures: i64,
    loop_guard_blocks: i64,
    waiting_sessions: usize,
) -> Vec<String> {
    let mut directives = Vec::new();
    if review_failures > 0 {
        directives.push(
            "Tighten completion discipline: rehearse or self-check work before Reflect hands it to the reviewer."
                .to_string(),
        );
    }
    if eval_failures > 0 {
        directives.push(
            "Protect operator-specific behavior: inspect recent eval regressions before changing runtime habits or prompts."
                .to_string(),
        );
    }
    if loop_guard_blocks > 0 {
        directives.push(
            "Reduce retry thrash: diversify tool plans earlier instead of repeating identical invocations."
                .to_string(),
        );
    }
    if waiting_sessions > 0 {
        directives.push(
            "Clarify blocked goals: promote prerequisites or satisfy wake conditions so sessions do not idle behind dependencies."
                .to_string(),
        );
    }
    if directives.is_empty() {
        directives.push(format!(
            "Recent sessions look stable across the last {session_count} runs. Keep the current operating pattern and watch for drift."
        ));
    }
    directives
}
