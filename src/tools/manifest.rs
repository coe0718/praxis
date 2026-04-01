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
