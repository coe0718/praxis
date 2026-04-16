use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Internal,
    Shell,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub kind: ToolKind,
    pub required_level: u8,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub rehearsal_required: bool,
    #[serde(default)]
    pub allowed_paths: Vec<String>,
    /// Read-path roots for `file-read` tools. Empty means data_dir only.
    #[serde(default)]
    pub allowed_read_paths: Vec<String>,
    /// Executable path for `shell` tools (e.g. `/usr/bin/git`).
    #[serde(default)]
    pub path: Option<String>,
    /// Fixed arguments prepended before any request-supplied arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Maximum wall-clock seconds to wait for the process (default 30).
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    // ---- HTTP tool fields ----
    /// URL template for `http` tools. Supports `{param}` placeholders
    /// substituted from the request payload's `params` map.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// HTTP method for `http` tools (GET, POST, PUT, DELETE, PATCH).
    /// Defaults to GET.
    #[serde(default)]
    pub method: Option<String>,
    /// Extra HTTP request headers as `"Key: Value"` strings.
    #[serde(default)]
    pub headers: Vec<String>,
    /// Static JSON body template. Payload params substitute `{param}` markers.
    #[serde(default)]
    pub body: Option<String>,
}

impl ToolManifest {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("tool name must not be empty");
        }

        if self.description.trim().is_empty() {
            bail!("tool {} must include a description", self.name);
        }

        if !(1..=4).contains(&self.required_level) {
            bail!("tool {} required_level must be between 1 and 4", self.name);
        }

        for scope in &self.allowed_paths {
            if scope.trim().is_empty() {
                bail!("tool {} allowed_paths must not contain blanks", self.name);
            }
        }

        Ok(())
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} [{}] level={} approval={} rehearsal={}",
            self.name,
            self.kind.as_str(),
            self.required_level,
            self.requires_approval,
            self.rehearsal_required
        )
    }
}

impl ToolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolKind::Internal => "internal",
            ToolKind::Shell => "shell",
            ToolKind::Http => "http",
        }
    }
}
