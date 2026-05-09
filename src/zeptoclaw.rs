//! ZeptoClaw - 32+ Built-in Tools for Praxis.
//!
//! Expanded tool inventory beyond the core 4 (file-read, git-query, shell-exec, web-fetch).
//! Each tool is a self-contained capability with minimal dependencies.

use serde::{Deserialize, Serialize};

/// Built-in tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub id: String,
    pub description: String,
    pub category: ToolCategory,
    pub requires_auth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    FileSystem,
    Network,
    Data,
    Communication,
    Development,
    Utility,
}

/// Registry of all built-in tools.
pub struct ToolInventory {
    tools: Vec<ToolDef>,
}

impl ToolInventory {
    pub fn new() -> Self {
        Self {
            tools: vec![
                // Core 4 (already exist)
                ToolDef {
                    id: "file-read".into(),
                    description: "Read file contents".into(),
                    category: ToolCategory::FileSystem,
                    requires_auth: false,
                },
                ToolDef {
                    id: "git-query".into(),
                    description: "Query git state".into(),
                    category: ToolCategory::Development,
                    requires_auth: false,
                },
                ToolDef {
                    id: "shell-exec".into(),
                    description: "Execute shell command".into(),
                    category: ToolCategory::Utility,
                    requires_auth: true,
                },
                ToolDef {
                    id: "web-fetch".into(),
                    description: "HTTP GET request".into(),
                    category: ToolCategory::Network,
                    requires_auth: false,
                },
                // FileSystem (5-8)
                ToolDef {
                    id: "file-write".into(),
                    description: "Write to file".into(),
                    category: ToolCategory::FileSystem,
                    requires_auth: true,
                },
                ToolDef {
                    id: "file-list".into(),
                    description: "List directory contents".into(),
                    category: ToolCategory::FileSystem,
                    requires_auth: false,
                },
                ToolDef {
                    id: "file-search".into(),
                    description: "Search for files by pattern".into(),
                    category: ToolCategory::FileSystem,
                    requires_auth: false,
                },
                ToolDef {
                    id: "file-delete".into(),
                    description: "Delete file or directory".into(),
                    category: ToolCategory::FileSystem,
                    requires_auth: true,
                },
                // Development (9-16)
                ToolDef {
                    id: "code-analyze".into(),
                    description: "Static code analysis".into(),
                    category: ToolCategory::Development,
                    requires_auth: false,
                },
                ToolDef {
                    id: "cargo-check".into(),
                    description: "Run cargo check on Rust project".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "npm-install".into(),
                    description: "Install npm dependencies".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "docker-run".into(),
                    description: "Run Docker container".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "git-commit".into(),
                    description: "Create git commit".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "git-push".into(),
                    description: "Push to git remote".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "env-var".into(),
                    description: "Get/set environment variable".into(),
                    category: ToolCategory::Development,
                    requires_auth: true,
                },
                ToolDef {
                    id: "http-post".into(),
                    description: "HTTP POST request".into(),
                    category: ToolCategory::Network,
                    requires_auth: false,
                },
                // Communication (17-20)
                ToolDef {
                    id: "email-send".into(),
                    description: "Send email".into(),
                    category: ToolCategory::Communication,
                    requires_auth: true,
                },
                ToolDef {
                    id: "sms-send".into(),
                    description: "Send SMS message".into(),
                    category: ToolCategory::Communication,
                    requires_auth: true,
                },
                ToolDef {
                    id: "discord-send".into(),
                    description: "Send Discord message".into(),
                    category: ToolCategory::Communication,
                    requires_auth: true,
                },
                ToolDef {
                    id: "slack-post".into(),
                    description: "Post to Slack channel".into(),
                    category: ToolCategory::Communication,
                    requires_auth: true,
                },
                // Data (21-24)
                ToolDef {
                    id: "json-parse".into(),
                    description: "Parse and validate JSON".into(),
                    category: ToolCategory::Data,
                    requires_auth: false,
                },
                ToolDef {
                    id: "csv-read".into(),
                    description: "Read CSV file".into(),
                    category: ToolCategory::Data,
                    requires_auth: false,
                },
                ToolDef {
                    id: "sql-query".into(),
                    description: "Execute SQL query".into(),
                    category: ToolCategory::Data,
                    requires_auth: true,
                },
                ToolDef {
                    id: "embedding-create".into(),
                    description: "Create text embedding".into(),
                    category: ToolCategory::Data,
                    requires_auth: true,
                },
                // Utility (25-32)
                ToolDef {
                    id: "crypto-hash".into(),
                    description: "Compute cryptographic hash".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "time-now".into(),
                    description: "Get current timestamp".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "cron-parse".into(),
                    description: "Parse cron expression".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "url-encode".into(),
                    description: "URL encode/decode".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "base64-encode".into(),
                    description: "Base64 encode/decode".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "uuid-generate".into(),
                    description: "Generate UUID".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "random-int".into(),
                    description: "Generate random integer".into(),
                    category: ToolCategory::Utility,
                    requires_auth: false,
                },
                ToolDef {
                    id: "url-shorten".into(),
                    description: "Shorten URL".into(),
                    category: ToolCategory::Network,
                    requires_auth: false,
                },
            ],
        }
    }

    pub fn list_all(&self) -> &[ToolDef] {
        &self.tools
    }

    pub fn by_category(&self, cat: ToolCategory) -> Vec<&ToolDef> {
        self.tools.iter().filter(|t| t.category == cat).collect()
    }

    pub fn find(&self, id: &str) -> Option<&ToolDef> {
        self.tools.iter().find(|t| t.id == id)
    }
}

impl Default for ToolInventory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_count() {
        let inv = ToolInventory::new();
        assert_eq!(inv.list_all().len(), 32);
    }

    #[test]
    fn test_find_tool() {
        let inv = ToolInventory::new();
        assert!(inv.find("file-read").is_some());
        assert!(inv.find("nonexistent").is_none());
    }

    #[test]
    fn test_category_filter() {
        let inv = ToolInventory::new();
        let dev_tools = inv.by_category(ToolCategory::Development);
        assert!(!dev_tools.is_empty());
    }
}
