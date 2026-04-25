//! Persistent operator (user) memory — survives across sessions.
//!
//! Unlike agent-scoped hot/cold memory which decays and consolidates, user
//! memory is a persistent key-value store that the agent uses to remember
//! operator preferences, facts, corrections, and decisions.
//!
//! Backed by `user_memory.json` in the Praxis data directory.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A single entry in the operator's persistent memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMemoryEntry {
    pub key: String,
    pub value: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// The full operator memory store.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserMemory {
    pub entries: Vec<UserMemoryEntry>,
}

impl UserMemory {
    /// Load user memory from disk, or return an empty store if the file doesn't exist.
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
        let raw = serde_json::to_string_pretty(self).context("failed to serialize user memory")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Insert or update a memory entry by key.
    pub fn upsert(&mut self, key: &str, value: &str, tags: Vec<String>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == key) {
            entry.value = value.to_string();
            if !tags.is_empty() {
                entry.tags = tags;
            }
            entry.updated_at = now;
        } else {
            self.entries.push(UserMemoryEntry {
                key: key.to_string(),
                value: value.to_string(),
                tags,
                created_at: now.clone(),
                updated_at: now,
            });
        }
        Ok(())
    }

    /// Remove a memory entry by key. Returns true if something was removed.
    pub fn forget(&mut self, key: &str) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.key != key);
        self.entries.len() < len_before
    }

    /// Search entries by substring match on key, value, or tags.
    pub fn search(&self, query: &str) -> Vec<&UserMemoryEntry> {
        let lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.key.to_lowercase().contains(&lower)
                    || e.value.to_lowercase().contains(&lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&lower))
            })
            .collect()
    }

    /// Get a specific entry by exact key match.
    pub fn get(&self, key: &str) -> Option<&UserMemoryEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    /// List all keys.
    pub fn keys(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.key.as_str()).collect()
    }

    /// Render all entries as a markdown block for context injection.
    pub fn render(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut out = String::from("## User Memory\n\n");
        out.push_str("Operator preferences and facts remembered across sessions:\n\n");
        for entry in &self.entries {
            let tags = if entry.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", entry.tags.join(", "))
            };
            out.push_str(&format!("- **{}**: {}{}\n", entry.key, entry.value, tags));
        }
        out.trim().to_string()
    }
}

/// Execute a user memory action based on params from the tool call.
pub fn execute_user_memory_action(
    memory_path: &Path,
    action: &str,
    key: Option<&str>,
    value: Option<&str>,
    tags: Vec<String>,
) -> Result<String> {
    let mut memory = UserMemory::load(memory_path)?;

    match action {
        "upsert" => {
            let key =
                key.ok_or_else(|| anyhow::anyhow!("memory upsert requires 'key' parameter"))?;
            let value =
                value.ok_or_else(|| anyhow::anyhow!("memory upsert requires 'value' parameter"))?;
            memory.upsert(key, value, tags)?;
            memory.save(memory_path)?;
            Ok(format!("memory: upserted '{}'", key))
        }

        "search" => {
            let query = value.or(key).ok_or_else(|| {
                anyhow::anyhow!("memory search requires a query (use 'value' or 'key' param)")
            })?;
            let results = memory.search(query);
            if results.is_empty() {
                return Ok("memory: no matches".to_string());
            }
            let lines: Vec<String> = results
                .iter()
                .map(|e| {
                    let tag_str = if e.tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", e.tags.join(", "))
                    };
                    format!("  {} = {}{}", e.key, e.value, tag_str)
                })
                .collect();
            Ok(format!("memory: {} match(es)\n{}", lines.len(), lines.join("\n")))
        }

        "forget" => {
            let key =
                key.ok_or_else(|| anyhow::anyhow!("memory forget requires 'key' parameter"))?;
            if memory.forget(key) {
                memory.save(memory_path)?;
                Ok(format!("memory: forgot '{}'", key))
            } else {
                Ok(format!("memory: '{}' not found", key))
            }
        }

        "list" => {
            if memory.entries.is_empty() {
                return Ok("memory: (empty)".to_string());
            }
            let lines: Vec<String> =
                memory.entries.iter().map(|e| format!("  {}", e.key)).collect();
            Ok(format!("memory: {} entries\n{}", lines.len(), lines.join("\n")))
        }

        _ => {
            anyhow::bail!(
                "unknown memory action '{}'. Supported: upsert, search, forget, list",
                action
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn upsert_and_search() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("user_memory.json");

        execute_user_memory_action(
            &path,
            "upsert",
            Some("preferred_language"),
            Some("Rust"),
            vec!["preference".to_string()],
        )
        .unwrap();
        execute_user_memory_action(
            &path,
            "upsert",
            Some("timezone"),
            Some("America/New_York"),
            vec!["location".to_string()],
        )
        .unwrap();

        let result =
            execute_user_memory_action(&path, "search", None, Some("Rust"), vec![]).unwrap();
        assert!(result.contains("preferred_language"));
        assert!(result.contains("Rust"));
    }

    #[test]
    fn forget_removes_entry() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("user_memory.json");

        execute_user_memory_action(&path, "upsert", Some("temp"), Some("ephemeral"), vec![])
            .unwrap();
        execute_user_memory_action(&path, "forget", Some("temp"), None, vec![]).unwrap();

        let memory = UserMemory::load(&path).unwrap();
        assert!(memory.entries.is_empty());
    }

    #[test]
    fn upsert_overwrites_existing() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("user_memory.json");

        execute_user_memory_action(&path, "upsert", Some("editor"), Some("vim"), vec![]).unwrap();
        execute_user_memory_action(&path, "upsert", Some("editor"), Some("helix"), vec![]).unwrap();

        let memory = UserMemory::load(&path).unwrap();
        assert_eq!(memory.get("editor").unwrap().value, "helix");
    }

    #[test]
    fn list_shows_keys() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("user_memory.json");

        execute_user_memory_action(&path, "upsert", Some("a"), Some("1"), vec![]).unwrap();

        let result = execute_user_memory_action(&path, "list", None, None, vec![]).unwrap();
        assert!(result.contains("a"));
    }

    #[test]
    fn search_by_tag() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("user_memory.json");

        execute_user_memory_action(
            &path,
            "upsert",
            Some("editor"),
            Some("neovim"),
            vec!["tools".to_string(), "dev".to_string()],
        )
        .unwrap();

        let result =
            execute_user_memory_action(&path, "search", None, Some("tools"), vec![]).unwrap();
        assert!(result.contains("editor"));
    }
}
