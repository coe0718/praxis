use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use super::{Goal, GoalParser};

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

#[cfg(test)]
mod tests {
    use super::{GoalParser, MarkdownGoalParser};

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
                "# Goals\n\n- [ ] G-002: Ship dependent work\n  blocked_by: G-001, external-api\n  wake_when: env:PRAXIS_GO\n",
            )
            .unwrap();

        assert_eq!(goals[0].blocked_by, vec!["G-001", "external-api"]);
        assert_eq!(goals[0].wake_when.as_deref(), Some("env:PRAXIS_GO"));
    }
}
