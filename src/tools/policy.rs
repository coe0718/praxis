use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{config::AppConfig, paths::PraxisPaths, storage::StoredApprovalRequest};

use super::ToolManifest;

const MAX_WRITE_PATHS_PER_REQUEST: usize = 8;

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
    matches!(path.to_string_lossy().as_ref(), "praxis.toml" | ".env") || path.starts_with("tools")
}
