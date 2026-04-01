use std::{fs, path::Path};

use anyhow::{Context, Result};

use crate::{
    config::AppConfig,
    identity::Goal,
    memory::{MemoryLoader, MemoryStore},
    paths::PraxisPaths,
};

use super::{BudgetedContext, ContextBudgeter, ContextSourceInput};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalContextLoader;

impl LocalContextLoader {
    pub fn load<S: MemoryStore>(
        &self,
        config: &AppConfig,
        paths: &PraxisPaths,
        store: &S,
        requested_task: Option<&str>,
        open_goals: &[Goal],
    ) -> Result<BudgetedContext> {
        let memory = MemoryLoader.load(store, requested_task, open_goals)?;
        let inputs = vec![
            source("identity", read_file(&paths.identity_file)?),
            source("active_goals", render_goals(open_goals)),
            source("memory_hot", memory.render_hot()),
            source("memory_cold", memory.render_cold()),
            source("patterns", read_file(&paths.patterns_file)?),
            source("journal", tail_lines(&read_file(&paths.journal_file)?, 12)),
            source("task", requested_task.unwrap_or_default().to_string()),
        ];

        Ok(ContextBudgeter.allocate(config, inputs))
    }
}

fn source(name: &str, content: String) -> ContextSourceInput {
    ContextSourceInput {
        source: name.to_string(),
        content,
    }
}

fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

fn render_goals(goals: &[Goal]) -> String {
    goals
        .iter()
        .map(|goal| format!("- {}: {}", goal.id, goal.title))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tail_lines(content: &str, limit: usize) -> String {
    let lines = content.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(limit);
    lines[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use crate::{
        config::AppConfig,
        identity::{Goal, IdentityPolicy, LocalIdentityPolicy},
        memory::{MemoryStore, NewHotMemory},
        paths::PraxisPaths,
        storage::{SessionStore, SqliteSessionStore},
    };

    use super::LocalContextLoader;

    #[test]
    fn loads_budgeted_context_with_memory_sources() {
        let temp = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
        let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
        LocalIdentityPolicy
            .ensure_foundation(
                &paths,
                &config,
                chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            )
            .unwrap();

        let store = SqliteSessionStore::new(paths.database_file.clone());
        store.initialize().unwrap();
        store
            .insert_hot_memory(NewHotMemory {
                content: "Operator prefers a CLI-first workflow".to_string(),
                summary: Some("CLI-first workflow".to_string()),
                importance: 0.9,
                tags: vec!["operator".to_string()],
                expires_at: None,
            })
            .unwrap();

        let context = LocalContextLoader
            .load(
                &config,
                &paths,
                &store,
                Some("improve local memory search"),
                &[Goal {
                    id: "G-001".to_string(),
                    title: "Ship memory foundation".to_string(),
                    completed: false,
                    line_number: 1,
                }],
            )
            .unwrap();

        assert!(context.render().contains("memory_hot"));
    }
}
