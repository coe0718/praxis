use anyhow::Result;

use crate::{
    config::AppConfig,
    identity::Goal,
    memory::{MemoryLoader, MemoryStore, OperationalMemoryLoader},
    paths::PraxisPaths,
    state::SessionState,
    storage::{AnatomyStore, OperationalMemoryStore},
};

use super::{BudgetedContext, ContextBudgeter, ContextSourceInput, TrackedContextReader};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalContextLoader;

pub(crate) struct ContextLoadRequest<'a> {
    pub config: &'a AppConfig,
    pub paths: &'a PraxisPaths,
    pub state: &'a mut SessionState,
    pub tool_summary: &'a str,
    pub requested_task: Option<&'a str>,
    pub open_goals: &'a [Goal],
}

impl LocalContextLoader {
    pub fn load<S: MemoryStore + OperationalMemoryStore + AnatomyStore>(
        &self,
        store: &S,
        request: ContextLoadRequest<'_>,
    ) -> Result<BudgetedContext> {
        let ContextLoadRequest {
            config,
            paths,
            state,
            tool_summary,
            requested_task,
            open_goals,
        } = request;
        let memory = MemoryLoader.load(store, requested_task, open_goals)?;
        let operational = OperationalMemoryLoader.load(store, requested_task, open_goals)?;
        let reader = TrackedContextReader;
        let inputs = vec![
            source(
                "identity",
                reader.read(store, state, &paths.identity_file, "identity")?,
            ),
            source("active_goals", render_goals(open_goals)),
            source("do_not_repeat", operational.render_do_not_repeat()),
            source("known_bugs", operational.render_known_bugs()),
            source("memory_hot", memory.render_hot()),
            source("memory_cold", memory.render_cold()),
            source(
                "patterns",
                reader.read(store, state, &paths.patterns_file, "patterns")?,
            ),
            source(
                "journal",
                tail_lines(
                    &reader.read(store, state, &paths.journal_file, "journal")?,
                    12,
                ),
            ),
            source("tools", tool_summary.to_string()),
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
        state::SessionState,
        storage::{SessionStore, SqliteSessionStore},
    };

    use super::{ContextLoadRequest, LocalContextLoader};

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
        let mut state = SessionState::new(
            chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            Some("improve local memory search".to_string()),
        );

        let context = LocalContextLoader
            .load(
                &store,
                ContextLoadRequest {
                    config: &config,
                    paths: &paths,
                    state: &mut state,
                    tool_summary:
                        "- internal-maintenance [internal] level=1 approval=false rehearsal=false",
                    requested_task: Some("improve local memory search"),
                    open_goals: &[Goal {
                        id: "G-001".to_string(),
                        title: "Ship memory foundation".to_string(),
                        completed: false,
                        line_number: 1,
                        blocked_by: Vec::new(),
                        wake_when: None,
                    }],
                },
            )
            .unwrap();

        assert!(context.render().contains("memory_hot"));
    }

    #[test]
    fn repeated_reads_fall_back_to_anatomy_summaries() {
        let temp = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
        let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
        LocalIdentityPolicy
            .ensure_foundation(
                &paths,
                &config,
                chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            )
            .unwrap();

        let store = SqliteSessionStore::new(paths.database_file.clone());
        store.initialize().unwrap();
        let mut state = SessionState::new(
            chrono::Utc.with_ymd_and_hms(2026, 4, 3, 12, 0, 0).unwrap(),
            None,
        );

        let first = LocalContextLoader
            .load(
                &store,
                ContextLoadRequest {
                    config: &config,
                    paths: &paths,
                    state: &mut state,
                    tool_summary: "tools",
                    requested_task: None,
                    open_goals: &[],
                },
            )
            .unwrap();
        let second = LocalContextLoader
            .load(
                &store,
                ContextLoadRequest {
                    config: &config,
                    paths: &paths,
                    state: &mut state,
                    tool_summary: "tools",
                    requested_task: None,
                    open_goals: &[],
                },
            )
            .unwrap();

        assert!(first.render().contains("## identity"));
        assert!(second.render().contains("Repeated read avoided."));
        assert!(state.repeated_reads_avoided >= 3);
    }
}
