use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

const BOUNDARIES_HEADING: &str = "## Boundaries";
const REVIEW_INTERVAL_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct BoundaryReviewState {
    pub last_confirmed_at: Option<String>,
    pub last_note: Option<String>,
}

impl BoundaryReviewState {
    pub fn load_or_default(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let state: Self = serde_json::from_str(&raw)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;
        state.validate()?;
        Ok(state)
    }

    pub fn save_if_missing(&self, path: &Path) -> Result<()> {
        if path.exists() {
            return Ok(());
        }
        self.save(path)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .context("failed to serialize boundary review state")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(last_confirmed_at) = &self.last_confirmed_at {
            DateTime::parse_from_rfc3339(last_confirmed_at).with_context(|| {
                format!("invalid boundary review timestamp {last_confirmed_at}")
            })?;
        }
        Ok(())
    }

    pub fn review_due(&self, now: DateTime<Utc>) -> bool {
        self.last_confirmed_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc) + Duration::days(REVIEW_INTERVAL_DAYS) <= now)
            .unwrap_or(true)
    }
}

pub fn list_boundaries(path: &Path) -> Result<Vec<String>> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(section_items(&contents, BOUNDARIES_HEADING))
}

pub fn add_boundary(path: &Path, rule: &str) -> Result<()> {
    let rule = rule.trim();
    if rule.is_empty() {
        bail!("boundary rule must not be empty");
    }
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let updated = insert_under_heading(&contents, BOUNDARIES_HEADING, rule);
    fs::write(path, updated).with_context(|| format!("failed to write {}", path.display()))
}

pub fn confirm_review(
    path: &Path,
    now: DateTime<Utc>,
    note: Option<&str>,
) -> Result<BoundaryReviewState> {
    let state = BoundaryReviewState {
        last_confirmed_at: Some(now.to_rfc3339()),
        last_note: note.map(str::trim).filter(|value| !value.is_empty()).map(ToString::to_string),
    };
    state.save(path)?;
    Ok(state)
}

pub fn review_prompt(state: &BoundaryReviewState, now: DateTime<Utc>) -> Option<String> {
    state
        .review_due(now)
        .then(|| "Weekly alignment review: have any hard limits changed?".to_string())
}

fn section_items(contents: &str, heading: &str) -> Vec<String> {
    let mut in_section = false;
    let mut items = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section && trimmed.starts_with("- ") {
            items.push(trimmed.trim_start_matches("- ").to_string());
        }
    }
    items
}

fn insert_under_heading(contents: &str, heading: &str, item: &str) -> String {
    let mut lines = contents.lines().map(str::to_string).collect::<Vec<_>>();
    let Some(index) = lines.iter().position(|line| line.trim() == heading) else {
        let mut updated = contents.trim_end().to_string();
        updated.push_str(&format!("\n\n{heading}\n- {item}\n"));
        return updated;
    };
    let mut insert_at = lines[index + 1..]
        .iter()
        .position(|line| line.starts_with("## "))
        .map(|offset| index + 1 + offset)
        .unwrap_or(lines.len());
    while insert_at > index + 1 && lines[insert_at - 1].trim().is_empty() {
        insert_at -= 1;
    }
    lines.insert(insert_at, format!("- {item}"));
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{BoundaryReviewState, insert_under_heading, review_prompt, section_items};

    #[test]
    fn inserts_rules_under_boundaries_heading() {
        let contents = "# Identity\n\n## Boundaries\n- Existing\n\n## Other\n- Later\n";
        let updated = insert_under_heading(contents, "## Boundaries", "New rule");

        assert!(updated.contains("- Existing\n- New rule\n\n## Other"));
    }

    #[test]
    fn review_due_defaults_true_until_confirmed() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 12, 12, 0, 0).unwrap();
        let empty = BoundaryReviewState::default();
        assert!(empty.review_due(now));

        let confirmed = BoundaryReviewState {
            last_confirmed_at: Some("2026-04-10T12:00:00Z".to_string()),
            last_note: None,
        };
        assert!(!confirmed.review_due(now));
        assert!(review_prompt(&confirmed, now).is_none());
    }

    #[test]
    fn lists_only_boundary_section_items() {
        let contents = "# Identity\n\n## Boundaries\n- A\n- B\n\n## Other\n- C\n";
        assert_eq!(section_items(contents, "## Boundaries"), vec!["A", "B"]);
    }
}
