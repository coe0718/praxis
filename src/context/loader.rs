use std::{fs, path::Path};

use anyhow::Result;
use serde::Deserialize;

use crate::{
    config::AppConfig,
    identity::Goal,
    memory::{MemoryLinkStore, MemoryLoader, MemoryStore, OperationalMemoryLoader},
    paths::PraxisPaths,
    skills,
    state::SessionState,
    storage::{AnatomyStore, DecisionReceiptStore, OperationalMemoryStore},
};

use super::{
    BudgetedContext, ContextBudgeter, ContextSourceInput, TrackedContextReader, adapt_config,
};

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
    pub fn load<
        S: MemoryStore + MemoryLinkStore + OperationalMemoryStore + AnatomyStore + DecisionReceiptStore,
    >(
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
        let config = adapt_config(config, &paths.context_adaptation_file)?;
        let memory = MemoryLoader.load(store, requested_task, open_goals)?;
        let operational = OperationalMemoryLoader.load(store, requested_task, open_goals)?;
        let recent_decisions = store.recent_decisions(5)?;
        let reader = TrackedContextReader;
        let inputs = vec![
            source(
                "soul",
                reader.read(store, state, &paths.soul_file, "soul")?,
            ),
            source(
                "identity",
                reader.read(store, state, &paths.identity_file, "identity")?,
            ),
            source("operator_model", load_operator_model(&paths.operator_model_file)),
            source(
                "agents",
                reader.read(store, state, &paths.agents_file, "agents")?,
            ),
            source("active_goals", render_goals(open_goals)),
            source("do_not_repeat", operational.render_do_not_repeat()),
            source("known_bugs", operational.render_known_bugs()),
            source("memory_hot", memory.render_hot()),
            source("memory_cold", memory.render_cold()),
            source("memory_linked", memory.render_linked()),
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
            source("tools", render_tools(tool_summary, &paths.skills_dir)),
            source("decision_receipts", render_receipts(&recent_decisions)),
            source("task", requested_task.unwrap_or_default().to_string()),
        ];

        Ok(ContextBudgeter.allocate(&config, inputs))
    }
}

/// Combine the tool manifest summary with the compact skill catalog.
fn render_tools(tool_summary: &str, skills_dir: &Path) -> String {
    let catalog = skills::render_catalog(skills_dir);
    if catalog.is_empty() {
        return tool_summary.to_string();
    }
    format!("{tool_summary}\n\n{catalog}")
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

/// Load and render the dialectic operator model from `operator_model.json`.
/// Returns an empty string if the file is absent or cannot be parsed, so the
/// loader degrades gracefully on instances that pre-date the SOUL/IDENTITY split.
fn load_operator_model(path: &Path) -> String {
    #[derive(Deserialize)]
    struct OperatorModel {
        #[serde(default)]
        working_hypotheses: Vec<String>,
        #[serde(default)]
        confirmed_traits: Vec<String>,
        #[serde(default)]
        tensions: Vec<String>,
        #[serde(default)]
        counter_evidence: Vec<String>,
    }

    let Ok(raw) = fs::read_to_string(path) else {
        return String::new();
    };
    let Ok(model) = serde_json::from_str::<OperatorModel>(&raw) else {
        return String::new();
    };

    if model.working_hypotheses.is_empty()
        && model.confirmed_traits.is_empty()
        && model.tensions.is_empty()
        && model.counter_evidence.is_empty()
    {
        return String::new();
    }

    let mut out = String::from("## Operator Model\n");
    if !model.confirmed_traits.is_empty() {
        out.push_str("\n### Confirmed Traits\n");
        for t in &model.confirmed_traits {
            out.push_str(&format!("- {t}\n"));
        }
    }
    if !model.working_hypotheses.is_empty() {
        out.push_str("\n### Working Hypotheses\n");
        for h in &model.working_hypotheses {
            out.push_str(&format!("- {h}\n"));
        }
    }
    if !model.tensions.is_empty() {
        out.push_str("\n### Tensions\n");
        for t in &model.tensions {
            out.push_str(&format!("- {t}\n"));
        }
    }
    if !model.counter_evidence.is_empty() {
        out.push_str("\n### Counter-Evidence\n");
        for c in &model.counter_evidence {
            out.push_str(&format!("- {c}\n"));
        }
    }
    out
}

fn render_receipts(receipts: &[crate::storage::StoredDecisionReceipt]) -> String {
    if receipts.is_empty() {
        return String::new();
    }
    receipts
        .iter()
        .map(|r| {
            let goal = r
                .goal_id
                .as_deref()
                .map(|id| format!(" goal={id}"))
                .unwrap_or_default();
            let approval = if r.approval_required { " approval=required" } else { "" };
            format!(
                "[{:.0}%]{}{} {} — {}",
                r.confidence * 100.0,
                goal,
                approval,
                r.reason_code,
                truncate(&r.chosen_action, 120),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
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
                        parent_id: None,
                        blocked_by: Vec::new(),
                        wake_when: None,
                    }],
                },
            )
            .unwrap();

        assert!(context.render().contains("## agents"));
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
