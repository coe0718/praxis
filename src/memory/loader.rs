use crate::identity::Goal;

use super::{MemoryLinkStore, MemoryStore, MemoryTier, StoredMemory, build_lookup_query};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMemoryContext {
    pub hot: Vec<StoredMemory>,
    pub cold: Vec<StoredMemory>,
    /// Memories surfaced via relational links from the top hot memories.
    pub linked: Vec<StoredMemory>,
    pub query: Option<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryLoader;

impl MemoryLoader {
    pub fn load<S: MemoryStore + MemoryLinkStore>(
        &self,
        store: &S,
        requested_task: Option<&str>,
        open_goals: &[Goal],
    ) -> anyhow::Result<LoadedMemoryContext> {
        let query = build_lookup_query(requested_task, open_goals);
        let (hot, cold) = if let Some(query) = query.as_deref() {
            let searched = store.search_memories(query, 6)?;
            let hot = searched
                .iter()
                .filter(|m| m.tier == MemoryTier::Hot)
                .cloned()
                .collect::<Vec<_>>();
            let cold = searched
                .iter()
                .filter(|m| m.tier == MemoryTier::Cold)
                .cloned()
                .collect::<Vec<_>>();

            if hot.is_empty() && cold.is_empty() {
                (store.recent_hot_memories(3)?, store.strongest_cold_memories(3)?)
            } else {
                (hot, cold)
            }
        } else {
            (store.recent_hot_memories(3)?, store.strongest_cold_memories(3)?)
        };

        // Expand the top 3 hot memories through their relational links.
        let mut linked_ids: std::collections::HashSet<i64> = hot.iter().map(|m| m.id).collect();
        linked_ids.extend(cold.iter().map(|m| m.id));
        let mut linked = Vec::new();
        for memory in hot.iter().take(3) {
            for related in store.linked_memories(memory.id, 2)? {
                if !linked_ids.contains(&related.id) {
                    linked_ids.insert(related.id);
                    linked.push(related);
                }
            }
        }

        Ok(LoadedMemoryContext { hot, cold, linked, query })
    }
}

impl LoadedMemoryContext {
    pub fn render_hot(&self) -> String {
        render_memories(&self.hot)
    }

    pub fn render_cold(&self) -> String {
        render_memories(&self.cold)
    }

    pub fn render_linked(&self) -> String {
        render_memories(&self.linked)
    }
}

fn render_memories(memories: &[StoredMemory]) -> String {
    memories
        .iter()
        .map(|memory| {
            let summary = memory.summary.as_deref().unwrap_or(memory.content.as_str());
            if memory.tags.is_empty() {
                format!("[{}] {}", memory.id, summary)
            } else {
                format!("[{}] {} [{}]", memory.id, summary, memory.tags.join(", "))
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
            linked: vec![],
            query: None,
        };
        assert_eq!(context.render_hot(), "");
        assert_eq!(context.render_linked(), "");
    }
}
