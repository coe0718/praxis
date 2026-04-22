use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::ToolManifest;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolRequestPayload {
    #[serde(default)]
    pub append_text: Option<String>,
    /// Key-value params substituted into HTTP endpoint/body templates.
    #[serde(default)]
    pub params: std::collections::HashMap<String, String>,
    /// Raw JSON body override for HTTP tools (takes precedence over `body` template).
    #[serde(default)]
    pub body: Option<String>,
}

pub fn build_payload(
    manifest: &ToolManifest,
    append_text: Option<String>,
    params: HashMap<String, String>,
) -> Result<Option<String>> {
    match manifest.name.as_str() {
        "praxis-data-write" => {
            if !params.is_empty() {
                bail!("tool {} does not accept --param", manifest.name);
            }
            let text = append_text
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("tool {} requires --append-text", manifest.name))?;
            let payload = ToolRequestPayload {
                append_text: Some(text),
                params: Default::default(),
                body: None,
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        "shell-exec" => {
            if append_text.is_some() {
                bail!("tool shell-exec does not accept --append-text; use --param command=<cmd>");
            }
            if !params.contains_key("command") {
                bail!("tool shell-exec requires --param command=<cmd>");
            }
            let payload = ToolRequestPayload {
                append_text: None,
                params,
                body: None,
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        "file-read" => {
            if append_text.is_some() {
                bail!("tool file-read does not accept --append-text; use --param path=<path>");
            }
            if !params.contains_key("path") {
                bail!("tool file-read requires --param path=<path>");
            }
            let payload = ToolRequestPayload {
                append_text: None,
                params,
                body: None,
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        "web-fetch" => {
            if append_text.is_some() {
                bail!("tool web-fetch does not accept --append-text; use --param url=<url>");
            }
            if !params.contains_key("url") {
                bail!("tool web-fetch requires --param url=<url>");
            }
            let payload = ToolRequestPayload {
                append_text: None,
                params,
                body: None,
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        "git-query" => {
            if append_text.is_some() {
                bail!("tool git-query does not accept --append-text; use --param args=<git args>");
            }
            if !params.contains_key("args") {
                bail!("tool git-query requires --param args=<git subcommand and flags>");
            }
            let payload = ToolRequestPayload {
                append_text: None,
                params,
                body: None,
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        _ => {
            if append_text.is_some() {
                bail!("tool {} does not accept --append-text", manifest.name);
            }
            if !params.is_empty() {
                let payload = ToolRequestPayload {
                    append_text: None,
                    params,
                    body: None,
                };
                return serde_json::to_string(&payload)
                    .map(Some)
                    .context("failed to serialize tool request payload");
            }
            Ok(None)
        }
    }
}

pub fn parse_payload(raw: Option<&str>) -> Result<ToolRequestPayload> {
    match raw {
        Some(raw) => serde_json::from_str(raw).context("failed to parse tool request payload"),
        None => Ok(ToolRequestPayload::default()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{build_payload, parse_payload};
    use crate::tools::{ToolKind, ToolManifest};

    fn write_manifest() -> ToolManifest {
        ToolManifest {
            name: "praxis-data-write".to_string(),
            description: "append notes".to_string(),
            kind: ToolKind::Shell,
            required_level: 2,
            requires_approval: true,
            rehearsal_required: true,
            allowed_paths: vec!["JOURNAL.md".to_string()],
            allowed_read_paths: Vec::new(),
            path: None,
            args: Vec::new(),
            timeout_secs: None,
            endpoint: None,
            method: None,
            headers: Vec::new(),
            body: None,
            allowed_vault_keys: None,
            allowed_oauth_providers: None,
        }
    }

    #[test]
    fn requires_append_text_for_data_write() {
        let error = build_payload(&write_manifest(), None, HashMap::new())
            .unwrap_err()
            .to_string();
        assert!(error.contains("--append-text"));
    }

    #[test]
    fn parses_append_payload() {
        let raw =
            build_payload(&write_manifest(), Some("hello".to_string()), HashMap::new()).unwrap();
        let payload = parse_payload(raw.as_deref()).unwrap();

        assert_eq!(payload.append_text.as_deref(), Some("hello"));
    }
}
