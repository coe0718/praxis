use std::fs;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{
    boundaries::{BoundaryReviewState, review_prompt},
    events::read_events_since,
    memory::MemoryStore,
    oauth::{GmailClient, OAuthTokenStore},
    paths::PraxisPaths,
    storage::{ApprovalStatus, ApprovalStore, SqliteSessionStore},
};

/// Generate a morning brief for the operator.
///
/// Aggregates: active goals, recent hot memories, pending approvals,
/// boundary review status, and recent events — sized for a Telegram message.
pub fn generate_brief(paths: &PraxisPaths, now: DateTime<Utc>) -> Result<String> {
    let mut sections: Vec<String> = Vec::new();

    // ── Greeting ──────────────────────────────────────────────────────────
    let greeting = greeting_for_hour(now.hour());
    sections.push(format!("🌅 {} — Morning Brief", greeting));

    // ── Active goals ──────────────────────────────────────────────────────
    let goals_section = active_goals_section(paths)?;
    if !goals_section.is_empty() {
        sections.push(goals_section);
    }

    // ── Hot memories (top 3) ──────────────────────────────────────────────
    let memory_section = hot_memories_section(paths)?;
    if !memory_section.is_empty() {
        sections.push(memory_section);
    }

    // ── Pending approvals ─────────────────────────────────────────────────
    let approval_section = pending_approvals_section(paths)?;
    if !approval_section.is_empty() {
        sections.push(approval_section);
    }

    // ── Boundary review ───────────────────────────────────────────────────
    let boundary_state = BoundaryReviewState::load_or_default(&paths.boundary_review_file)?;
    if let Some(prompt) = review_prompt(&boundary_state, now) {
        sections.push(format!("⚠️  {}", prompt));
    }

    // ── Recent events (last 5) ────────────────────────────────────────────
    let events_section = recent_events_section(paths)?;
    if !events_section.is_empty() {
        sections.push(events_section);
    }

    // ── Gmail inbox ───────────────────────────────────────────────────────
    let gmail_section = gmail_section(paths);
    if !gmail_section.is_empty() {
        sections.push(gmail_section);
    }

    // ── Journal excerpt ───────────────────────────────────────────────────
    let journal_section = journal_excerpt_section(paths)?;
    if !journal_section.is_empty() {
        sections.push(journal_section);
    }

    Ok(sections.join("\n\n"))
}

fn greeting_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=17 => "Good afternoon",
        18..=21 => "Good evening",
        _ => "Hello",
    }
}

fn active_goals_section(paths: &PraxisPaths) -> Result<String> {
    let content = match fs::read_to_string(&paths.goals_file) {
        Ok(c) => c,
        Err(_) => return Ok(String::new()),
    };

    let active: Vec<&str> = content
        .lines()
        .filter(|line| line.trim_start().starts_with("- [ ]"))
        .take(5)
        .collect();

    if active.is_empty() {
        return Ok(String::new());
    }

    let mut out = "📋 Active goals:".to_string();
    for goal in active {
        let trimmed = goal.trim_start_matches("- [ ]").trim();
        out.push_str(&format!("\n• {}", trimmed));
    }
    Ok(out)
}

fn hot_memories_section(paths: &PraxisPaths) -> Result<String> {
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let memories = store.recent_hot_memories(3)?;
    if memories.is_empty() {
        return Ok(String::new());
    }

    let mut out = "🧠 Top memories:".to_string();
    for m in &memories {
        let text = m.summary.as_deref().unwrap_or(m.content.as_str());
        let truncated = if text.len() > 100 {
            format!("{}…", &text[..100])
        } else {
            text.to_string()
        };
        out.push_str(&format!("\n• {}", truncated));
    }
    Ok(out)
}

fn pending_approvals_section(paths: &PraxisPaths) -> Result<String> {
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let approvals = store.list_approvals(Some(ApprovalStatus::Pending))?;
    if approvals.is_empty() {
        return Ok(String::new());
    }

    let mut out = format!("⏳ {} pending approval(s):", approvals.len());
    for a in approvals.iter().take(3) {
        let summary = if a.summary.len() > 60 {
            format!("{}…", &a.summary[..60])
        } else {
            a.summary.clone()
        };
        out.push_str(&format!("\n• [#{}] {}", a.id, summary));
    }
    if approvals.len() > 3 {
        out.push_str(&format!("\n  … and {} more. Use /queue to view.", approvals.len() - 3));
    }
    Ok(out)
}

fn recent_events_section(paths: &PraxisPaths) -> Result<String> {
    let (events, _) = read_events_since(&paths.events_file, 0)?;
    let recent: Vec<_> = events.into_iter().rev().take(5).collect();
    if recent.is_empty() {
        return Ok(String::new());
    }

    let mut out = "📡 Recent events:".to_string();
    for e in recent.into_iter().rev() {
        out.push_str(&format!("\n• [{}] {}", e.kind, e.detail));
    }
    Ok(out)
}

fn journal_excerpt_section(paths: &PraxisPaths) -> Result<String> {
    let content = match fs::read_to_string(&paths.journal_file) {
        Ok(c) => c,
        Err(_) => return Ok(String::new()),
    };

    let recent_lines: Vec<&str> =
        content.lines().rev().filter(|line| !line.trim().is_empty()).take(4).collect();

    if recent_lines.is_empty() {
        return Ok(String::new());
    }

    let mut out = "📓 Journal:".to_string();
    for line in recent_lines.into_iter().rev() {
        out.push_str(&format!("\n{}", line));
    }
    Ok(out)
}

fn gmail_section(paths: &PraxisPaths) -> String {
    let store = OAuthTokenStore::new(&paths.data_dir);
    let client = match GmailClient::from_store(&store) {
        Ok(Some(c)) => c,
        _ => return String::new(),
    };
    match client.list_recent(5) {
        Ok(emails) if !emails.is_empty() => {
            let mut out = "✉️  Unread emails:".to_string();
            for e in &emails {
                let from = if e.from.len() > 40 {
                    format!("{}…", &e.from[..40])
                } else {
                    e.from.clone()
                };
                let subject = if e.subject.len() > 60 {
                    format!("{}…", &e.subject[..60])
                } else {
                    e.subject.clone()
                };
                out.push_str(&format!("\n• {from}: {subject}"));
            }
            out
        }
        Ok(_) => String::new(),
        Err(e) => {
            log::debug!("gmail brief section failed: {e}");
            String::new()
        }
    }
}

trait HourExt {
    fn hour(&self) -> u32;
}

impl HourExt for DateTime<Utc> {
    fn hour(&self) -> u32 {
        chrono::Timelike::hour(self)
    }
}

#[cfg(test)]
mod tests {
    use super::greeting_for_hour;

    #[test]
    fn greeting_by_hour() {
        assert_eq!(greeting_for_hour(6), "Good morning");
        assert_eq!(greeting_for_hour(14), "Good afternoon");
        assert_eq!(greeting_for_hour(20), "Good evening");
        assert_eq!(greeting_for_hour(2), "Hello");
    }
}
