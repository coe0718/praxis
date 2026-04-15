use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result, bail};

use crate::{oauth::OAuthTokenStore, paths::PraxisPaths, storage::StoredApprovalRequest};

use super::{ToolKind, ToolManifest, policy::normalize_relative, request::parse_payload};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionResult {
    pub summary: String,
}

pub fn execute_request(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    match manifest.name.as_str() {
        "internal-maintenance" => Ok(ToolExecutionResult {
            summary: "Internal maintenance completed without external side effects.".to_string(),
        }),
        "praxis-data-write" => append_text(paths, manifest, request),
        _ => match manifest.kind {
            ToolKind::Shell
                if manifest
                    .path
                    .as_deref()
                    .is_some_and(|p| !p.trim().is_empty()) =>
            {
                run_shell(paths, manifest, request)
            }
            _ => Ok(fallback_result(
                manifest,
                request,
                "No execution adapter is installed for this tool yet.",
            )),
        },
    }
}

fn run_shell(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let exe = manifest
        .path
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "shell tool {} has no 'path' field in its manifest",
                manifest.name
            )
        })?;

    let timeout_secs = manifest
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .max(1)
        .min(300);

    // Parse any extra arguments from the structured payload.
    let payload = parse_payload(request.payload_json.as_deref())?;
    let extra_args: Vec<String> = payload
        .append_text
        .as_deref()
        .map(|text| {
            text.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    log::info!(
        "executing shell tool {} (timeout {}s): {} {:?}",
        manifest.name,
        timeout_secs,
        exe,
        &manifest.args
    );

    let mut cmd = Command::new(exe);
    cmd.args(&manifest.args);
    if !extra_args.is_empty() {
        cmd.args(&extra_args);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Inject OAuth tokens as env vars so shell tools can call authenticated APIs.
    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if !token.is_expired() {
                let key = format!("PRAXIS_OAUTH_{}_TOKEN", provider.to_uppercase());
                cmd.env(key, &token.access_token);
            }
        }
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn shell tool {}", manifest.name))?;

    // Poll with a deadline so we can kill runaway processes.
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let pid = child.id();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                bail!(
                    "shell tool {} timed out after {}s (pid {pid})",
                    manifest.name,
                    timeout_secs
                );
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to collect output of shell tool {}", manifest.name))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let detail = if stderr.is_empty() {
            stdout.chars().take(200).collect::<String>()
        } else {
            stderr.chars().take(200).collect::<String>()
        };
        bail!(
            "shell tool {} exited with code {code}: {detail}",
            manifest.name
        );
    }

    let output_snippet = if stdout.is_empty() {
        "(no output)".to_string()
    } else {
        stdout.chars().take(500).collect::<String>()
    };

    Ok(ToolExecutionResult {
        summary: format!(
            "Shell tool {} completed successfully.\n{}",
            manifest.name, output_snippet
        ),
    })
}

fn append_text(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let Some(raw_payload) = request.payload_json.as_deref() else {
        return Ok(fallback_result(
            manifest,
            request,
            "Legacy approved request had no structured payload, so Praxis kept this run safe and skipped the file mutation.",
        ));
    };
    let payload = parse_payload(Some(raw_payload))?;
    let text = payload
        .append_text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| anyhow::anyhow!("tool {} requires append_text payload", manifest.name))?;

    let plans = request
        .write_paths
        .iter()
        .map(|raw| plan_append(paths, raw, text))
        .collect::<Result<Vec<_>>>()?;

    for plan in plans {
        if let Some(block) = plan.block.as_deref() {
            append_block(&plan.path, block)?;
        }
    }

    Ok(ToolExecutionResult {
        summary: format!(
            "Executed approved tool {} and appended operator-approved text to {} file(s).",
            manifest.name,
            request.write_paths.len()
        ),
    })
}

#[derive(Debug)]
struct AppendPlan {
    path: PathBuf,
    block: Option<String>,
}

fn plan_append(paths: &PraxisPaths, raw: &str, text: &str) -> Result<AppendPlan> {
    let relative = normalize_relative(raw)?;
    ensure_safe_destination(&paths.data_dir, &relative)?;
    let full_path = paths.data_dir.join(&relative);
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Ok(AppendPlan {
        block: render_append_block(&full_path, text)?,
        path: full_path,
    })
}

fn ensure_safe_destination(root: &Path, relative: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in relative.components() {
        current.push(component.as_os_str());
        let candidate = root.join(&current);
        if !candidate.exists() {
            continue;
        }
        let metadata = fs::symlink_metadata(&candidate)
            .with_context(|| format!("failed to inspect {}", candidate.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "refusing to write through symlink target {}",
                candidate.display()
            );
        }
    }
    Ok(())
}

fn render_append_block(path: &Path, text: &str) -> Result<Option<String>> {
    let existing = match fs::read(path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()));
        }
    };
    let mut block = String::new();
    if !existing.is_empty() && !existing.ends_with(b"\n") {
        block.push('\n');
    }
    block.push_str(text);
    block.push('\n');
    if existing.ends_with(block.as_bytes()) {
        Ok(None)
    } else {
        Ok(Some(block))
    }
}

fn append_block(path: &Path, block: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    write!(file, "{block}").with_context(|| format!("failed to append {}", path.display()))?;
    Ok(())
}

fn fallback_result(
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
    reason: &str,
) -> ToolExecutionResult {
    let rehearsal = if manifest.rehearsal_required {
        "Rehearsal required; "
    } else {
        ""
    };
    ToolExecutionResult {
        summary: format!(
            "{rehearsal}{reason} Stub execution recorded for approved tool {} with {} declared write paths. No external side effects were performed.",
            manifest.name,
            request.write_paths.len()
        ),
    }
}
