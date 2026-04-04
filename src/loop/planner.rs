use std::{collections::HashSet, path::Path};

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::identity::Goal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalDecision {
    Selected(Goal),
    Waiting(String),
    Complete,
}

pub fn choose_goal(goals: &[Goal], data_dir: &Path, now: DateTime<Utc>) -> Result<GoalDecision> {
    let open_goals = goals
        .iter()
        .filter(|goal| !goal.completed)
        .cloned()
        .collect::<Vec<_>>();
    if open_goals.is_empty() {
        return Ok(GoalDecision::Complete);
    }

    let completed_ids = goals
        .iter()
        .filter(|goal| goal.completed)
        .map(|goal| goal.id.clone())
        .collect::<HashSet<_>>();
    let active_parent_ids = open_goals
        .iter()
        .filter_map(|goal| goal.parent_id.clone())
        .collect::<HashSet<_>>();
    let goal_ids = goals
        .iter()
        .map(|goal| goal.id.clone())
        .collect::<HashSet<_>>();
    let dependent_counts = dependency_counts(&open_goals);
    let mut ready = open_goals
        .iter()
        .filter(|goal| !active_parent_ids.contains(&goal.id))
        .filter(|goal| !is_blocked(goal, &completed_ids, &goal_ids))
        .filter(|goal| wake_condition_met(goal, data_dir, now))
        .cloned()
        .collect::<Vec<_>>();

    ready.sort_by(|left, right| {
        let left_count = dependent_counts
            .get(left.id.as_str())
            .copied()
            .unwrap_or_default();
        let right_count = dependent_counts
            .get(right.id.as_str())
            .copied()
            .unwrap_or_default();
        right_count
            .cmp(&left_count)
            .then(left.line_number.cmp(&right.line_number))
    });

    if let Some(goal) = ready.into_iter().next() {
        return Ok(GoalDecision::Selected(goal));
    }

    Ok(GoalDecision::Waiting(format!(
        "{} open goals are waiting on dependencies or triggers.",
        open_goals.len()
    )))
}

fn dependency_counts(goals: &[Goal]) -> std::collections::HashMap<&str, usize> {
    let mut counts = std::collections::HashMap::new();
    for goal in goals {
        for dependency in &goal.blocked_by {
            *counts.entry(dependency.as_str()).or_insert(0) += 1;
        }
    }
    counts
}

fn is_blocked(goal: &Goal, completed_ids: &HashSet<String>, goal_ids: &HashSet<String>) -> bool {
    goal.blocked_by.iter().any(|dependency| {
        if goal_ids.contains(dependency) {
            !completed_ids.contains(dependency)
        } else {
            true
        }
    })
}

fn wake_condition_met(goal: &Goal, data_dir: &Path, now: DateTime<Utc>) -> bool {
    let Some(condition) = goal.wake_when.as_deref() else {
        return true;
    };

    if let Some(path) = condition.strip_prefix("file_exists:") {
        return resolve_path(data_dir, path).exists();
    }
    if let Some(var) = condition.strip_prefix("env:") {
        return std::env::var_os(var).is_some();
    }
    if let Some(timestamp) = condition.strip_prefix("after:") {
        return DateTime::parse_from_rfc3339(timestamp)
            .map(|value| value.with_timezone(&Utc) <= now)
            .unwrap_or(false);
    }

    false
}

fn resolve_path(base: &Path, candidate: &str) -> std::path::PathBuf {
    let path = Path::new(candidate);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{GoalDecision, choose_goal};
    use crate::identity::Goal;

    #[test]
    fn prefers_unblocking_goal_with_dependents() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
        let goals = vec![
            Goal {
                id: "G-002".to_string(),
                title: "Dependent".to_string(),
                completed: false,
                line_number: 1,
                parent_id: None,
                blocked_by: vec!["G-001".to_string()],
                wake_when: None,
            },
            Goal {
                id: "G-001".to_string(),
                title: "Dependency".to_string(),
                completed: false,
                line_number: 2,
                parent_id: None,
                blocked_by: Vec::new(),
                wake_when: None,
            },
        ];

        let selected = choose_goal(&goals, std::path::Path::new("."), now).unwrap();
        assert_eq!(
            selected,
            GoalDecision::Selected(Goal {
                id: "G-001".to_string(),
                title: "Dependency".to_string(),
                completed: false,
                line_number: 2,
                parent_id: None,
                blocked_by: Vec::new(),
                wake_when: None,
            })
        );
    }

    #[test]
    fn prefers_child_before_open_parent_goal() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap();
        let goals = vec![
            Goal {
                id: "G-010".to_string(),
                title: "Parent".to_string(),
                completed: false,
                line_number: 1,
                parent_id: None,
                blocked_by: Vec::new(),
                wake_when: None,
            },
            Goal {
                id: "G-011".to_string(),
                title: "Child".to_string(),
                completed: false,
                line_number: 2,
                parent_id: Some("G-010".to_string()),
                blocked_by: Vec::new(),
                wake_when: None,
            },
        ];

        let selected = choose_goal(&goals, std::path::Path::new("."), now).unwrap();
        assert_eq!(selected, GoalDecision::Selected(goals[1].clone()));
    }
}
