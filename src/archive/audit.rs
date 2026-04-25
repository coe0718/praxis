use std::{fs, path::Path, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};

use crate::paths::PraxisPaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditExportSummary {
    pub output_file: PathBuf,
    pub session_count: usize,
    pub approval_count: usize,
}

pub fn export_audit(
    paths: &PraxisPaths,
    output_file: &Path,
    overwrite: bool,
    now: DateTime<Utc>,
    days: i64,
) -> Result<AuditExportSummary> {
    if output_file.exists() && !overwrite {
        anyhow::bail!("{} already exists; pass --overwrite to replace it", output_file.display());
    }
    if let Some(parent) = output_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let cutoff = (now - Duration::days(days.max(0))).to_rfc3339();
    let connection = Connection::open(&paths.database_file)
        .with_context(|| format!("failed to open {}", paths.database_file.display()))?;
    let sessions = query_lines(
        &connection,
        "
        SELECT ended_at,
               outcome,
               COALESCE(selected_goal_id, '-'),
               COALESCE(selected_task, '-'),
               action_summary
        FROM sessions
        WHERE ended_at >= ?1
        ORDER BY ended_at DESC
        LIMIT 20
        ",
        &cutoff,
        |row| {
            Ok(format!(
                "- {} outcome={} goal={} task={} summary={}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?
            ))
        },
    )?;
    let approvals = query_lines(
        &connection,
        "
        SELECT updated_at, tool_name, status, summary
        FROM approval_requests
        WHERE updated_at >= ?1
        ORDER BY updated_at DESC
        LIMIT 20
        ",
        &cutoff,
        |row| {
            Ok(format!(
                "- {} tool={} status={} summary={}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?
            ))
        },
    )?;
    let reviews = grouped_counts(&connection, "review_runs", "reviewed_at", &cutoff)?;
    let evals = grouped_counts(&connection, "eval_runs", "evaluated_at", &cutoff)?;
    let providers = provider_lines(&connection, &cutoff)?;
    let memory = memory_lines(&connection, &cutoff)?;
    let events = tail_lines(&paths.events_file, 20)?;

    let report = format!(
        "# Praxis Audit Export\n\n- generated_at: {}\n- data_dir: {}\n- cutoff: {}\n- file_diffs: not_captured\n\n## Sessions\n{}\n\n## Approvals\n{}\n\n## Reviews\n{}\n\n## Evals\n{}\n\n## Provider Usage\n{}\n\n## Memory Writes\n{}\n\n## Recent Events\n{}\n",
        now.to_rfc3339(),
        paths.data_dir.display(),
        cutoff,
        section(&sessions, "No recent sessions."),
        section(&approvals, "No recent approvals."),
        section(&reviews, "No recent reviews."),
        section(&evals, "No recent evals."),
        section(&providers, "No recent provider usage."),
        section(&memory, "No recent memory writes."),
        section(&events, "No recent events."),
    );

    fs::write(output_file, report)
        .with_context(|| format!("failed to write {}", output_file.display()))?;
    Ok(AuditExportSummary {
        output_file: output_file.to_path_buf(),
        session_count: sessions.len(),
        approval_count: approvals.len(),
    })
}

fn query_lines<T>(
    connection: &Connection,
    sql: &str,
    cutoff: &str,
    map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut statement = connection.prepare(sql).context("failed to prepare query")?;
    let rows = statement.query_map(params![cutoff], map).context("failed to execute query")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to collect rows")
}

fn grouped_counts(
    connection: &Connection,
    table: &str,
    column: &str,
    cutoff: &str,
) -> Result<Vec<String>> {
    query_lines(
        connection,
        &format!(
            "SELECT status, COUNT(*) FROM {table} WHERE {column} >= ?1 GROUP BY status ORDER BY status"
        ),
        cutoff,
        |row| Ok(format!("- status={} count={}", row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    )
}

fn provider_lines(connection: &Connection, cutoff: &str) -> Result<Vec<String>> {
    query_lines(
        connection,
        "
        SELECT provider,
               COUNT(*),
               SUM(input_tokens + output_tokens),
               SUM(estimated_cost_micros)
        FROM provider_attempts
        WHERE session_id IN (SELECT id FROM sessions WHERE ended_at >= ?1)
        GROUP BY provider
        ORDER BY COUNT(*) DESC, provider ASC
        ",
        cutoff,
        |row| {
            Ok(format!(
                "- provider={} attempts={} tokens={} cost_micros={}",
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?
            ))
        },
    )
}

fn memory_lines(connection: &Connection, cutoff: &str) -> Result<Vec<String>> {
    let mut lines = query_lines(
        connection,
        "SELECT created_at, summary FROM hot_memories WHERE created_at >= ?1 ORDER BY created_at DESC LIMIT 10",
        cutoff,
        |row| {
            Ok(format!(
                "- hot {} {}",
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "summary=none".to_string())
            ))
        },
    )?;
    lines.extend(query_lines(
        connection,
        "SELECT created_at, statement FROM do_not_repeat WHERE created_at >= ?1 ORDER BY created_at DESC LIMIT 10",
        cutoff,
        |row| Ok(format!("- do_not_repeat {} {}", row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?);
    lines.extend(query_lines(
        connection,
        "SELECT last_seen_at, signature FROM known_bugs WHERE last_seen_at >= ?1 ORDER BY last_seen_at DESC LIMIT 10",
        cutoff,
        |row| Ok(format!("- known_bug {} {}", row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )?);
    Ok(lines)
}

fn tail_lines(path: &Path, limit: usize) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let lines = content
        .lines()
        .rev()
        .take(limit)
        .map(|line| format!("- {line}"))
        .collect::<Vec<_>>();
    Ok(lines.into_iter().rev().collect())
}

fn section(lines: &[String], empty: &str) -> String {
    if lines.is_empty() {
        empty.to_string()
    } else {
        lines.join("\n")
    }
}
