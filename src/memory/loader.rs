use crate::identity::Goal;

use super::{MemoryStore, MemoryTier, StoredMemory, build_lookup_query};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMemoryContext {
    pub hot: Vec<StoredMemory>,
    pub cold: Vec<StoredMemory>,
    pub query: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryLoader;

impl MemoryLoader {
    pub fn load<S: MemoryStore>(
        &self,
        store: &S,
        requested_task: Option<&str>,
        open_goals: &[Goal],
    ) -> anyhow::Result<LoadedMemoryContext> {
        let query = build_lookup_query(requested_task, open_goals);
        if let Some(query) = query.as_deref() {
            let searched = store.search_memories(query, 6)?;
            let hot = searched
                .iter()
                .filter(|memory| memory.tier == MemoryTier::Hot)
                .cloned()
                .collect::<Vec<_>>();
            let cold = searched
                .iter()
                .filter(|memory| memory.tier == MemoryTier::Cold)
                .cloned()
                .collect::<Vec<_>>();

            if !hot.is_empty() || !cold.is_empty() {
                return Ok(LoadedMemoryContext {
                    hot,
                    cold,
                    query: Some(query.to_string()),
                });
            }
        }

        Ok(LoadedMemoryContext {
            hot: store.recent_hot_memories(3)?,
            cold: store.strongest_cold_memories(3)?,
            query,
        })
    }
}

impl LoadedMemoryContext {
    pub fn render_hot(&self) -> String {
        render_memories(&self.hot)
    }

    pub fn render_cold(&self) -> String {
        render_memories(&self.cold)
    }
}

fn render_memories(memories: &[StoredMemory]) -> String {
    memories
        .iter()
        .map(|memory| {
            let summary = memory.summary.as_deref().unwrap_or(memory.content.as_str());
            if memory.tags.is_empty() {
                format!("- {}", summary)
            } else {
                format!("- {} [{}]", summary, memory.tags.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::LoadedMemoryContext;
    use crate::{identity::Goal, memory::build_lookup_query};

    #[test]
    fn builds_query_from_task_or_goals() {
        let from_task = build_lookup_query(Some("ship memory"), &[]);
        assert_eq!(from_task.as_deref(), Some("ship memory"));

        let from_goals = build_lookup_query(
            None,
            &[Goal {
                id: "G-001".to_string(),
                title: "Ship memory foundation".to_string(),
                completed: false,
                line_number: 1,
                parent_id: None,
                blocked_by: Vec::new(),
                wake_when: None,
            }],
        );
        assert!(from_goals.unwrap().contains("memory foundation"));
    }

    #[test]
    fn renders_loaded_memories() {
        let context = LoadedMemoryContext {
            hot: vec![],
            cold: vec![],
            query: None,
        };

        assert_eq!(context.render_hot(), "");
    }
}
