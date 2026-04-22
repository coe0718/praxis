use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{config::AppConfig, paths::PraxisPaths, storage::StoredApprovalRequest};

use super::{ToolManifest, request::parse_payload};

const MAX_WRITE_PATHS_PER_REQUEST: usize = 8;
const MAX_PROTECTED_WRITES_PER_REQUEST: usize = 2;
const MAX_APPEND_TEXT_BYTES: usize = 4 * 1024;

#[derive(Debug, Default, Clone, Copy)]
pub struct SecurityPolicy;

impl SecurityPolicy {
    pub fn validate_request(
        &self,
        config: &AppConfig,
        paths: &PraxisPaths,
        manifest: &ToolManifest,
        request: &StoredApprovalRequest,
    ) -> Result<()> {
        if manifest.required_level > config.security.level {
            bail!(
                "tool {} requires security level {}, but Praxis is configured for level {}",
                manifest.name,
                manifest.required_level,
                config.security.level
            );
        }

        if request.write_paths.len() > MAX_WRITE_PATHS_PER_REQUEST {
            bail!(
                "tool request {} exceeds the write-path circuit breaker limit of {}",
                request.id,
                MAX_WRITE_PATHS_PER_REQUEST
            );
        }
        validate_payload_size(manifest, request)?;

        let mut protected_writes = 0;
        let mut unique_paths = HashSet::new();
        for requested in &request.write_paths {
            if !unique_paths.insert(requested) {
                bail!(
                    "tool request {} repeats write path {}",
                    request.id,
                    requested
                );
            }

            let requested_path = normalize_relative(requested)?;
            if is_protected_path(&requested_path) {
                protected_writes += 1;
            }
            if is_locked_path(&requested_path) {
                bail!(
                    "tool request {} targets locked path {}",
                    request.id,
                    requested
                );
            }

            if !path_allowed(&requested_path, &manifest.allowed_paths)? {
                bail!(
                    "tool {} is not allowed to write {}",
                    manifest.name,
                    requested_path.display()
                );
            }

            let full_path = paths.data_dir.join(&requested_path);
            if !full_path.starts_with(&paths.data_dir) {
                bail!(
                    "tool request {} escapes the Praxis data directory",
                    request.id
                );
            }
        }
        if protected_writes > MAX_PROTECTED_WRITES_PER_REQUEST {
            bail!(
                "tool request {} exceeds the protected-file circuit breaker limit of {}",
                request.id,
                MAX_PROTECTED_WRITES_PER_REQUEST
            );
        }

        Ok(())
    }
}

fn path_allowed(requested: &Path, scopes: &[String]) -> Result<bool> {
    if scopes.is_empty() {
        return Ok(requested.as_os_str().is_empty());
    }

    for scope in scopes {
        let allowed = normalize_relative(scope)?;
        if requested == allowed || requested.starts_with(&allowed) {
            return Ok(true);
        }
    }

    Ok(false)
}

pub(crate) fn normalize_relative(raw: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute() {
        bail!("absolute paths are not allowed: {raw}");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                if !normalized.pop() {
                    bail!("path escapes the allowed root: {raw}");
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                bail!("absolute paths are not allowed: {raw}");
            }
        }
    }

    Ok(normalized)
}

fn is_locked_path(path: &Path) -> bool {
    matches!(
        path.to_string_lossy().as_ref(),
        "SOUL.md" | "praxis.toml" | ".env" | "hooks.toml"
    ) || path.starts_with("tools")
}

fn is_protected_path(path: &Path) -> bool {
    matches!(
        path.to_string_lossy().as_ref(),
        "IDENTITY.md"
            | "GOALS.md"
            | "ROADMAP.md"
            | "JOURNAL.md"
            | "METRICS.md"
            | "PATTERNS.md"
            | "LEARNINGS.md"
            | "AGENTS.md"
            | "CAPABILITIES.md"
            | "PROPOSALS.md"
    )
}

fn validate_payload_size(manifest: &ToolManifest, request: &StoredApprovalRequest) -> Result<()> {
    if manifest.name != "praxis-data-write" {
        return Ok(());
    }
    let Some(raw_payload) = request.payload_json.as_deref() else {
        return Ok(());
    };
    let payload = parse_payload(Some(raw_payload))?;
    let Some(text) = payload.append_text.as_deref() else {
        return Ok(());
    };
    if text.len() > MAX_APPEND_TEXT_BYTES {
        bail!(
            "tool request {} exceeds the append-size circuit breaker limit of {} bytes",
            request.id,
            MAX_APPEND_TEXT_BYTES
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        config::AppConfig,
        paths::PraxisPaths,
        storage::{ApprovalStatus, StoredApprovalRequest},
    };

    use super::{SecurityPolicy, ToolManifest};

    fn manifest(paths: &[&str]) -> ToolManifest {
        ToolManifest {
            name: "praxis-data-write".to_string(),
            description: "append notes".to_string(),
            kind: crate::tools::ToolKind::Shell,
            required_level: 2,
            requires_approval: true,
            rehearsal_required: true,
            allowed_paths: paths.iter().map(|path| path.to_string()).collect(),
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

    fn request(paths: &[&str], append_text: String) -> StoredApprovalRequest {
        StoredApprovalRequest {
            id: 7,
            tool_name: "praxis-data-write".to_string(),
            summary: "append note".to_string(),
            requested_by: "operator".to_string(),
            write_paths: paths.iter().map(|path| path.to_string()).collect(),
            payload_json: Some(format!("{{\"append_text\":{append_text:?}}}")),
            status: ApprovalStatus::Approved,
            status_note: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn rejects_requests_that_touch_too_many_protected_files() {
        let config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        let paths = PraxisPaths::for_data_dir("/tmp/praxis".into());
        let error = SecurityPolicy
            .validate_request(
                &config,
                &paths,
                &manifest(&["JOURNAL.md", "PROPOSALS.md", "METRICS.md"]),
                &request(
                    &["JOURNAL.md", "PROPOSALS.md", "METRICS.md"],
                    "safe note".to_string(),
                ),
            )
            .unwrap_err()
            .to_string();

        assert!(error.contains("protected-file circuit breaker"));
    }

    #[test]
    fn rejects_oversized_append_payloads() {
        let config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        let paths = PraxisPaths::for_data_dir("/tmp/praxis".into());
        let error = SecurityPolicy
            .validate_request(
                &config,
                &paths,
                &manifest(&["JOURNAL.md"]),
                &request(&["JOURNAL.md"], "A".repeat(4 * 1024 + 1)),
            )
            .unwrap_err()
            .to_string();

        assert!(error.contains("append-size circuit breaker"));
    }
}
