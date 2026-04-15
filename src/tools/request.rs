use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use super::ToolManifest;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolRequestPayload {
    #[serde(default)]
    pub append_text: Option<String>,
}

pub fn build_payload(
    manifest: &ToolManifest,
    append_text: Option<String>,
) -> Result<Option<String>> {
    match manifest.name.as_str() {
        "praxis-data-write" => {
            let text = append_text
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| anyhow::anyhow!("tool {} requires --append-text", manifest.name))?;
            let payload = ToolRequestPayload {
                append_text: Some(text),
            };
            serde_json::to_string(&payload)
                .map(Some)
                .context("failed to serialize tool request payload")
        }
        _ => {
            if append_text.is_some() {
                bail!("tool {} does not accept --append-text", manifest.name);
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
            path: None,
            args: Vec::new(),
            timeout_secs: None,
        }
    }

    #[test]
    fn requires_append_text_for_data_write() {
        let error = build_payload(&write_manifest(), None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("--append-text"));
    }

    #[test]
    fn parses_append_payload() {
        let raw = build_payload(&write_manifest(), Some("hello".to_string())).unwrap();
        let payload = parse_payload(raw.as_deref()).unwrap();

        assert_eq!(payload.append_text.as_deref(), Some("hello"));
    }
}
