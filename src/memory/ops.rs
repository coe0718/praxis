use serde::{Deserialize, Serialize};

use crate::{identity::Goal, storage::OperationalMemoryStore};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewDoNotRepeat {
    pub statement: String,
    pub tags: Vec<String>,
    pub severity: String,
    pub source_session_id: Option<i64>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDoNotRepeat {
    pub id: i64,
    pub statement: String,
    pub tags: Vec<String>,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewKnownBug {
    pub signature: String,
    pub symptoms: String,
    pub fix_summary: String,
    pub tags: Vec<String>,
    pub source_session_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredKnownBug {
    pub id: i64,
    pub signature: String,
    pub symptoms: String,
    pub fix_summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedOperationalContext {
    pub do_not_repeat: Vec<StoredDoNotRepeat>,
    pub known_bugs: Vec<StoredKnownBug>,
    pub query: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct OperationalMemoryLoader;

impl OperationalMemoryLoader {
    pub fn load<S: OperationalMemoryStore>(
        &self,
        store: &S,
        requested_task: Option<&str>,
        open_goals: &[Goal],
    ) -> anyhow::Result<LoadedOperationalContext> {
        let query = build_query(requested_task, open_goals);
        let (do_not_repeat, known_bugs) = match query.as_deref() {
            Some(query) => (
                store.search_do_not_repeat(query, 3)?,
                store.search_known_bugs(query, 3)?,
            ),
            None => (store.recent_do_not_repeat(2)?, store.recent_known_bugs(2)?),
        };

        Ok(LoadedOperationalContext {
            do_not_repeat,
            known_bugs,
            query,
        })
    }
}

impl LoadedOperationalContext {
    pub fn render_do_not_repeat(&self) -> String {
        self.do_not_repeat
            .iter()
            .map(|item| format!("- {} [{}]", item.statement, item.severity))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn render_known_bugs(&self) -> String {
        self.known_bugs
            .iter()
            .map(|bug| format!("- {} -> {}", bug.signature, bug.fix_summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn build_query(requested_task: Option<&str>, open_goals: &[Goal]) -> Option<String> {
    if let Some(task) = requested_task.filter(|task| !task.trim().is_empty()) {
        return Some(task.to_string());
    }

    open_goals
        .iter()
        .take(2)
        .map(|goal| goal.title.trim())
        .find(|title| !title.is_empty())
        .map(ToString::to_string)
}
