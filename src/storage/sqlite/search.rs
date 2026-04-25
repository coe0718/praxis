//! Session search SQLite implementation — LIKE-based search across
//! outcome, action_summary, selected_goal_title, and selected_task columns.

use anyhow::{Context, Result};

use crate::storage::{SessionSearchResult, StoredSession, search::SessionSearchStore};

use super::SqliteSessionStore;

impl SessionSearchStore for SqliteSessionStore {
    fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<SessionSearchResult>> {
        let conn = self.connect()?;
        let pattern = format!("%{}%", query);
        let limit_i64 = limit.min(50) as i64;

        let mut stmt = conn
            .prepare(
                "SELECT id, day, session_num, started_at, ended_at, outcome,
                        selected_goal_id, selected_goal_title, selected_task, action_summary
                 FROM sessions
                 WHERE outcome LIKE ?1
                    OR action_summary LIKE ?1
                    OR selected_goal_title LIKE ?1
                    OR selected_task LIKE ?1
                 ORDER BY ended_at DESC
                 LIMIT ?2",
            )
            .context("failed to prepare session search query")?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit_i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .context("failed to execute session search")?;

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for row_result in rows {
            let (
                id,
                day,
                session_num,
                started_at,
                ended_at,
                outcome,
                selected_goal_id,
                selected_goal_title,
                selected_task,
                action_summary,
            ) = row_result?;

            let (matched_column, snippet) = if outcome.to_lowercase().contains(&query_lower) {
                ("outcome", snippet_from(&outcome, &query_lower))
            } else if action_summary.to_lowercase().contains(&query_lower) {
                ("action_summary", snippet_from(&action_summary, &query_lower))
            } else if selected_goal_title
                .as_deref()
                .map(|g| g.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            {
                ("goal", snippet_from(selected_goal_title.as_deref().unwrap_or(""), &query_lower))
            } else {
                ("task", snippet_from(selected_task.as_deref().unwrap_or(""), &query_lower))
            };

            results.push(SessionSearchResult {
                session: StoredSession {
                    id,
                    day,
                    session_num,
                    started_at,
                    ended_at,
                    outcome,
                    selected_goal_id,
                    selected_goal_title,
                    selected_task,
                    action_summary,
                },
                matched_column: matched_column.to_string(),
                snippet,
            });
        }

        Ok(results)
    }
}

fn snippet_from(text: &str, query_lower: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(query_lower) {
        let start = pos.saturating_sub(40);
        let end = (pos + query_lower.len() + 60).min(text.len());
        let mut snippet: String = text.chars().skip(start).take(end - start).collect();
        if start > 0 {
            snippet.insert(0, '\u{2026}');
        }
        if end < text.len() {
            snippet.push('\u{2026}');
        }
        snippet
    } else {
        text.chars().take(200).collect()
    }
}
