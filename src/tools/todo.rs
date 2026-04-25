//! In-session task planner — lets the agent decompose complex tasks into
//! ordered subtasks and track progress within a session.
//!
//! The todo list is stored as a JSON file in the Praxis data directory.
//! It supports create, update, complete, cancel, and list operations.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// A single task item in the agent's per-session todo list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::InProgress => "in_progress",
            TodoStatus::Completed => "completed",
            TodoStatus::Cancelled => "cancelled",
        }
    }
}

/// A persisted task list for the agent's current work session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

impl TodoList {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if raw.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let raw = serde_json::to_string_pretty(self).context("failed to serialize todo list")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&TodoItem> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// Execute a todo tool command based on the action parameter.
pub fn execute_todo_action(
    todo_path: &Path,
    action: &str,
    id: Option<&str>,
    content: Option<&str>,
    status: Option<&str>,
) -> Result<String> {
    let mut list = TodoList::load(todo_path)?;

    match action {
        "create" => {
            let content = content
                .ok_or_else(|| anyhow::anyhow!("todo create requires 'content' parameter"))?;
            let id =
                id.map(String::from).unwrap_or_else(|| format!("task-{}", list.items.len() + 1));
            if list.items.iter().any(|i| i.id == id) {
                bail!("todo item with id '{id}' already exists");
            }
            list.items.push(TodoItem {
                id: id.clone(),
                content: content.to_string(),
                status: TodoStatus::Pending,
            });
            list.save(todo_path)?;
            Ok(format!("todo: created '{}' — {}", id, content))
        }

        "update" => {
            let id = id.ok_or_else(|| anyhow::anyhow!("todo update requires 'id' parameter"))?;
            let item = list
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| anyhow::anyhow!("todo item '{id}' not found"))?;

            if let Some(new_content) = content {
                item.content = new_content.to_string();
            }
            if let Some(new_status) = status {
                item.status = parse_status(new_status)?;
            }
            let status_str = item.status.as_str().to_string();
            list.save(todo_path)?;
            Ok(format!("todo: updated '{}' — status={}", id, status_str))
        }

        "complete" => {
            let id = id.ok_or_else(|| anyhow::anyhow!("todo complete requires 'id' parameter"))?;
            let item = list
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| anyhow::anyhow!("todo item '{id}' not found"))?;
            item.status = TodoStatus::Completed;
            list.save(todo_path)?;
            Ok(format!("todo: completed '{}'", id))
        }

        "cancel" => {
            let id = id.ok_or_else(|| anyhow::anyhow!("todo cancel requires 'id' parameter"))?;
            let item = list
                .items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| anyhow::anyhow!("todo item '{id}' not found"))?;
            item.status = TodoStatus::Cancelled;
            list.save(todo_path)?;
            Ok(format!("todo: cancelled '{}'", id))
        }

        "list" => {
            if list.items.is_empty() {
                return Ok("todo: (empty)".to_string());
            }
            let mut lines = Vec::new();
            for item in &list.items {
                lines.push(format!("  [{}] {} — {}", item.status.as_str(), item.id, item.content));
            }
            Ok(format!("todo:\n{}", lines.join("\n")))
        }

        _ => bail!(
            "unknown todo action '{}'. Supported: create, update, complete, cancel, list",
            action
        ),
    }
}

fn parse_status(raw: &str) -> Result<TodoStatus> {
    match raw {
        "pending" => Ok(TodoStatus::Pending),
        "in_progress" => Ok(TodoStatus::InProgress),
        "completed" => Ok(TodoStatus::Completed),
        "cancelled" => Ok(TodoStatus::Cancelled),
        _ => bail!(
            "unknown status '{}'. Supported: pending, in_progress, completed, cancelled",
            raw
        ),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn create_and_list() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        execute_todo_action(&path, "create", None, Some("Review pull requests"), None).unwrap();
        execute_todo_action(&path, "create", None, Some("Update dependencies"), None).unwrap();

        let list = TodoList::load(&path).unwrap();
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].status, TodoStatus::Pending);
    }

    #[test]
    fn complete_and_list() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        execute_todo_action(&path, "create", None, Some("Fix bug"), None).unwrap();
        execute_todo_action(&path, "complete", Some("task-1"), None, None).unwrap();

        let list = TodoList::load(&path).unwrap();
        assert_eq!(list.items[0].status, TodoStatus::Completed);
    }

    #[test]
    fn cancel_and_list() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        execute_todo_action(&path, "create", None, Some("Write tests"), None).unwrap();
        execute_todo_action(&path, "cancel", Some("task-1"), None, None).unwrap();

        let list = TodoList::load(&path).unwrap();
        assert_eq!(list.items[0].status, TodoStatus::Cancelled);
    }

    #[test]
    fn update_content() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        execute_todo_action(&path, "create", None, Some("Initial"), None).unwrap();
        execute_todo_action(&path, "update", Some("task-1"), Some("Updated content"), None)
            .unwrap();

        let list = TodoList::load(&path).unwrap();
        assert_eq!(list.items[0].content, "Updated content");
    }

    #[test]
    fn list_empty() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        let result = execute_todo_action(&path, "list", None, None, None).unwrap();
        assert!(result.contains("(empty)"));
    }

    #[test]
    fn duplicate_id_rejected() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("todo.json");

        execute_todo_action(&path, "create", Some("my-task"), Some("Task 1"), None).unwrap();
        let result = execute_todo_action(&path, "create", Some("my-task"), Some("Task 2"), None);
        assert!(result.is_err());
    }
}
