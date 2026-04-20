use serde_json::{Value, json};

use crate::paths::PraxisPaths;

use super::types::GoalRow;

pub(super) fn query_recent_sessions(
    db_path: &std::path::Path,
    limit: usize,
) -> anyhow::Result<Vec<Value>> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, day, session_num, started_at, ended_at, outcome,
                selected_goal_id, selected_goal_title, selected_task, action_summary
         FROM sessions ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit as i64], |row| {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "day": row.get::<_, i64>(1)?,
            "session_num": row.get::<_, i64>(2)?,
            "started_at": row.get::<_, String>(3)?,
            "ended_at": row.get::<_, String>(4)?,
            "outcome": row.get::<_, String>(5)?,
            "selected_goal_id": row.get::<_, Option<String>>(6)?,
            "selected_goal_title": row.get::<_, Option<String>>(7)?,
            "selected_task": row.get::<_, Option<String>>(8)?,
            "action_summary": row.get::<_, String>(9)?,
        }))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(anyhow::Error::from)
}

pub(super) fn parse_goals_file(path: &std::path::Path) -> anyhow::Result<Vec<GoalRow>> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut goals = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            let (raw_id, title) = split_goal_id(rest);
            goals.push(GoalRow {
                raw_id,
                title,
                completed: true,
            });
        } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
            let (raw_id, title) = split_goal_id(rest);
            goals.push(GoalRow {
                raw_id,
                title,
                completed: false,
            });
        }
    }
    Ok(goals)
}

pub(super) fn split_goal_id(s: &str) -> (String, String) {
    if let Some(colon) = s.find(": ") {
        let id = s[..colon].trim().to_string();
        let title = s[colon + 2..].trim().to_string();
        (id, title)
    } else {
        (String::new(), s.trim().to_string())
    }
}

pub(super) fn append_goal(path: &std::path::Path, description: &str) -> anyhow::Result<String> {
    let contents = std::fs::read_to_string(path).unwrap_or_default();
    let next_id = contents
        .lines()
        .filter_map(|l| l.split("G-").nth(1))
        .filter_map(|r| r.split(':').next())
        .filter_map(|d| d.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let goal_id = format!("G-{next_id:03}");
    let mut updated = contents.trim_end().to_string();
    updated.push_str(&format!("\n- [ ] {goal_id}: {description}\n"));
    std::fs::write(path, updated)?;
    Ok(goal_id)
}

pub(super) fn resolve_identity_file(paths: &PraxisPaths, name: &str) -> Option<std::path::PathBuf> {
    match name {
        "soul" => Some(paths.soul_file.clone()),
        "identity" => Some(paths.identity_file.clone()),
        "agents" => Some(paths.agents_file.clone()),
        "goals" => Some(paths.goals_file.clone()),
        "journal" => Some(paths.journal_file.clone()),
        "patterns" => Some(paths.patterns_file.clone()),
        "learnings" => Some(paths.learnings_file.clone()),
        "roadmap" => Some(paths.roadmap_file.clone()),
        _ => None,
    }
}

pub(super) fn read_jsonl_tail(path: &std::path::Path, limit: usize) -> anyhow::Result<Vec<Value>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path)?;
    let rows: Vec<Value> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let start = rows.len().saturating_sub(limit);
    Ok(rows[start..].to_vec())
}
