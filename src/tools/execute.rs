use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;

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
        "file-read" => read_file(paths, manifest, request),
        "shell-exec" => exec_shell_command(paths, manifest, request),
        _ => match manifest.kind {
            ToolKind::Shell
                if manifest
                    .path
                    .as_deref()
                    .is_some_and(|p| !p.trim().is_empty()) =>
            {
                run_shell(paths, manifest, request)
            }
            ToolKind::Http
                if manifest
                    .endpoint
                    .as_deref()
                    .is_some_and(|e| !e.trim().is_empty()) =>
            {
                run_http(paths, manifest, request)
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
        .params
        .get("args")
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .or_else(|| {
            payload.append_text.as_deref().map(|text| {
                text.split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
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

fn run_http(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let endpoint_template = manifest
        .endpoint
        .as_deref()
        .filter(|e| !e.trim().is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "http tool {} has no 'endpoint' field in its manifest",
                manifest.name
            )
        })?;

    let method = manifest
        .method
        .as_deref()
        .unwrap_or("GET")
        .to_ascii_uppercase();

    let timeout_secs = manifest
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .max(1)
        .min(120);

    let payload = parse_payload(request.payload_json.as_deref())?;

    // Substitute {param} placeholders in the URL.
    let url = substitute_params(endpoint_template, &payload.params);

    log::info!("executing http tool {} {} {}", manifest.name, method, url);

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("failed to build HTTP client for tool")?;

    let mut builder = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "PATCH" => client.patch(&url),
        other => bail!("http tool {} has unsupported method {other}", manifest.name),
    };

    // Apply manifest headers.
    for header_str in &manifest.headers {
        if let Some((key, value)) = header_str.split_once(':') {
            builder = builder.header(key.trim(), value.trim());
        }
    }

    // Inject OAuth tokens.
    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if !token.is_expired() {
                let header_name = format!("X-Praxis-OAuth-{}", provider_title(provider));
                builder = builder.header(header_name, &token.access_token);
            }
        }
    }

    // Attach body: explicit payload.body beats manifest.body template.
    let body_str = payload.body.clone().or_else(|| {
        manifest
            .body
            .as_ref()
            .map(|t| substitute_params(t, &payload.params))
    });

    if let Some(body) = body_str {
        builder = builder
            .header("Content-Type", "application/json")
            .body(body);
    }

    let response = builder
        .send()
        .with_context(|| format!("failed to send request for http tool {}", manifest.name))?;

    let status = response.status();
    let body_text = response
        .text()
        .unwrap_or_else(|_| "(unreadable body)".to_string());

    if !status.is_success() {
        bail!(
            "http tool {} returned {}: {}",
            manifest.name,
            status,
            body_text.chars().take(200).collect::<String>()
        );
    }

    let snippet = body_text.chars().take(500).collect::<String>();
    Ok(ToolExecutionResult {
        summary: format!(
            "HTTP tool {} completed ({}).\n{}",
            manifest.name, status, snippet
        ),
    })
}

/// Substitute `{key}` placeholders in `template` with values from `params`.
fn substitute_params(template: &str, params: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

/// Convert a provider name to Title-Case for use in header names.
fn provider_title(provider: &str) -> String {
    let mut chars = provider.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn read_file(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    const MAX_READ_BYTES: usize = 32 * 1024;

    let payload = parse_payload(request.payload_json.as_deref())?;
    let raw_path = payload
        .params
        .get("path")
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("file-read requires params.path"))?;

    let full_path = if manifest.allowed_read_paths.is_empty() {
        let relative = normalize_relative(raw_path)?;
        let candidate = paths.data_dir.join(&relative);
        if !candidate.starts_with(&paths.data_dir) {
            anyhow::bail!("file-read path escapes the Praxis data directory");
        }
        candidate
    } else {
        let input = Path::new(raw_path);
        let mut resolved: Option<PathBuf> = None;
        for root_str in &manifest.allowed_read_paths {
            let root = PathBuf::from(root_str);
            let candidate = if input.is_absolute() {
                input.to_path_buf()
            } else {
                root.join(input)
            };
            let Ok(canon_root) = fs::canonicalize(&root) else {
                continue;
            };
            let Ok(canon_candidate) = fs::canonicalize(&candidate) else {
                continue;
            };
            if canon_candidate.starts_with(&canon_root) {
                resolved = Some(candidate);
                break;
            }
        }
        resolved.ok_or_else(|| {
            anyhow::anyhow!(
                "file-read path {} is not within any allowed_read_paths root",
                raw_path
            )
        })?
    };

    if let Ok(meta) = fs::symlink_metadata(&full_path) {
        if meta.file_type().is_symlink() {
            anyhow::bail!(
                "file-read refuses to follow symlink at {}",
                full_path.display()
            );
        }
    }

    let content =
        fs::read(&full_path).with_context(|| format!("failed to read {}", full_path.display()))?;

    let truncated = content.len() > MAX_READ_BYTES;
    let text = String::from_utf8_lossy(&content[..content.len().min(MAX_READ_BYTES)]).to_string();
    let suffix = if truncated {
        format!("\n[truncated at {} bytes]", MAX_READ_BYTES)
    } else {
        String::new()
    };

    Ok(ToolExecutionResult {
        summary: format!(
            "file-read {} ({} bytes).\n{}{}",
            full_path.display(),
            content.len(),
            text,
            suffix
        ),
    })
}

fn exec_shell_command(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let command = payload
        .params
        .get("command")
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("shell-exec requires params.command"))?;

    let timeout_secs = manifest
        .timeout_secs
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
        .max(1)
        .min(300);

    log::info!(
        "executing shell-exec (timeout {}s): {}",
        timeout_secs,
        &command[..command.len().min(80)]
    );

    let mut cmd = Command::new("/bin/bash");
    cmd.args(["-c", command]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if !token.is_expired() {
                let key = format!("PRAXIS_OAUTH_{}_TOKEN", provider.to_uppercase());
                cmd.env(key, &token.access_token);
            }
        }
    }

    let mut child = cmd.spawn().context("failed to spawn shell-exec process")?;

    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let pid = child.id();
    loop {
        match child.try_wait()? {
            Some(_) => break,
            None if std::time::Instant::now() >= deadline => {
                let _ = child.kill();
                anyhow::bail!("shell-exec timed out after {}s (pid {pid})", timeout_secs);
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    let output = child
        .wait_with_output()
        .context("failed to collect shell-exec output")?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let detail = if stderr.is_empty() { &stdout } else { &stderr };
        anyhow::bail!(
            "shell-exec exited with code {code}: {}",
            detail.chars().take(200).collect::<String>()
        );
    }

    let snippet = if stdout.is_empty() {
        "(no output)".to_string()
    } else {
        stdout.chars().take(2000).collect::<String>()
    };

    Ok(ToolExecutionResult {
        summary: format!("shell-exec completed successfully.\n{snippet}"),
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
