use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use super::{Goal, GoalParser};

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownGoalParser;

impl GoalParser for MarkdownGoalParser {
    fn parse_goals(&self, content: &str) -> Result<Vec<Goal>> {
        let mut goals = Vec::new();

        for (index, raw_line) in content.lines().enumerate() {
            if let Some(goal) = parse_goal_line(raw_line, index + 1)? {
                goals.push(goal);
            }
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
    }))
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
}
