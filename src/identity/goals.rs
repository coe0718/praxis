use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use super::{Goal, GoalParser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalPromotion {
    pub goal_id: String,
    pub created: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownGoalParser;

impl GoalParser for MarkdownGoalParser {
    fn parse_goals(&self, content: &str) -> Result<Vec<Goal>> {
        let mut goals = Vec::new();
        let lines = content.lines().collect::<Vec<_>>();
        let mut index = 0;

        while index < lines.len() {
            if let Some(mut goal) = parse_goal_line(lines[index], index + 1)? {
                index += 1;
                while index < lines.len() {
                    let line = lines[index];
                    if parse_goal_line(line, index + 1)?.is_some() || !line.starts_with("  ") {
                        break;
                    }
                    apply_metadata(&mut goal, line, index + 1)?;
                    index += 1;
                }
                goals.push(goal);
                continue;
            }
            index += 1;
        }

        Ok(goals)
    }

    fn load_goals(&self, path: &Path) -> Result<Vec<Goal>> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        self.parse_goals(&raw)
    }
}

pub fn ensure_goal(path: &Path, title: &str) -> Result<GoalPromotion> {
    let parser = MarkdownGoalParser;
    let existing = if path.exists() {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        parser.parse_goals(&raw)?
    } else {
        Vec::new()
    };

    if let Some(goal) = existing.iter().find(|goal| goal.title == title) {
        return Ok(GoalPromotion {
            goal_id: goal.id.clone(),
            created: false,
        });
    }

    let goal_id = next_goal_id(&existing);
    let mut raw = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        "# Goals\n".to_string()
    };
    if !raw.ends_with('\n') {
        raw.push('\n');
    }
    if !raw.ends_with("\n\n") {
        raw.push('\n');
    }
    raw.push_str(&format!("- [ ] {goal_id}: {title}\n"));
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(GoalPromotion {
        goal_id,
        created: true,
    })
}

fn parse_goal_line(line: &str, line_number: usize) -> Result<Option<Goal>> {
    let (completed, remainder) = if let Some(rest) = line.strip_prefix("- [ ] ") {
        (false, rest)
    } else if let Some(rest) = line
        .strip_prefix("- [x] ")
        .or_else(|| line.strip_prefix("- [X] "))
    {
        (true, rest)
    } else {
        return Ok(None);
    };

    let (id, title) = remainder.split_once(':').with_context(|| {
        format!("invalid goal syntax on line {line_number}, expected G-001: title")
    })?;

    let id = id.trim().to_string();
    let title = title.trim().to_string();
    if id.is_empty() || title.is_empty() {
        bail!("invalid goal syntax on line {line_number}, expected non-empty id and title");
    }

    Ok(Some(Goal {
        id,
        title,
        completed,
        line_number,
        parent_id: None,
        blocked_by: Vec::new(),
        wake_when: None,
    }))
}

fn apply_metadata(goal: &mut Goal, line: &str, line_number: usize) -> Result<()> {
    let Some((key, value)) = line.trim().split_once(':') else {
        bail!("invalid goal metadata on line {line_number}, expected key: value");
    };
    let value = value.trim();
    match key.trim() {
        "blocked_by" => {
            goal.blocked_by = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect();
        }
        "parent" => {
            if value.is_empty() {
                bail!("parent metadata on line {line_number} must not be empty");
            }
            goal.parent_id = Some(value.to_string());
        }
        "wake_when" => {
            if value.is_empty() {
                bail!("wake_when metadata on line {line_number} must not be empty");
            }
            goal.wake_when = Some(value.to_string());
        }
        other => bail!("unsupported goal metadata key {other} on line {line_number}"),
    }
    Ok(())
}

fn next_goal_id(goals: &[Goal]) -> String {
    let next = goals
        .iter()
        .filter_map(|goal| goal.id.strip_prefix("G-"))
        .filter_map(|value| value.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    format!("G-{next:03}")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{GoalParser, MarkdownGoalParser, ensure_goal};

    #[test]
    fn parses_goal_markdown() {
        let parser = MarkdownGoalParser;
        let goals = parser
            .parse_goals("# Goals\n\n- [ ] G-001: Ship foundation\n- [x] G-002: Done already\n")
            .unwrap();

        assert_eq!(goals.len(), 2);
        assert_eq!(goals[0].id, "G-001");
        assert!(!goals[0].completed);
        assert!(goals[1].completed);
    }

    #[test]
    fn parses_goal_metadata() {
        let parser = MarkdownGoalParser;
        let goals = parser
            .parse_goals(
                "# Goals\n\n- [ ] G-002: Ship dependent work\n  parent: G-010\n  blocked_by: G-001, external-api\n  wake_when: env:PRAXIS_GO\n",
            )
            .unwrap();

        assert_eq!(goals[0].parent_id.as_deref(), Some("G-010"));
        assert_eq!(goals[0].blocked_by, vec!["G-001", "external-api"]);
        assert_eq!(goals[0].wake_when.as_deref(), Some("env:PRAXIS_GO"));
    }

    #[test]
    fn appends_new_goal_and_reuses_existing_title() {
        let temp = tempdir().unwrap();
        let goals_file = temp.path().join("GOALS.md");
        fs::write(&goals_file, "# Goals\n\n- [ ] G-001: Foundation\n").unwrap();

        let created = ensure_goal(&goals_file, "Automate recurring work").unwrap();
        let reused = ensure_goal(&goals_file, "Automate recurring work").unwrap();
        let raw = fs::read_to_string(&goals_file).unwrap();

        assert_eq!(created.goal_id, "G-002");
        assert!(created.created);
        assert_eq!(reused.goal_id, "G-002");
        assert!(!reused.created);
        assert_eq!(raw.matches("Automate recurring work").count(), 1);
    }
}
