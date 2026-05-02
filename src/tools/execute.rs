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

use crate::{
    oauth::OAuthTokenStore,
    paths::PraxisPaths,
    storage::StoredApprovalRequest,
    vault::Vault,
};

use super::{
    ToolKind, ToolManifest, clarify::execute_clarify, policy::normalize_relative,
    request::parse_payload, todo::execute_todo_action, vision::{VisionTool, VisionParameters},
};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Shell metacharacters that enable command injection when passed to `bash -c`.
const DANGEROUS_SHELL_CHARS: &[char] =
    &[';', '|', '&', '`', '$', '(', ')', '<', '>', '\n', '\r', '\\'];
const MAX_OUTPUT_BYTES: usize = 1024 * 1024; // 1 MB cap on command output

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolExecutionResult {
    pub summary: String,
}

pub fn execute_request(
    paths: &PraxisPaths,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let vault = Vault::load(&paths.vault_file).unwrap_or_default();
    match manifest.name.as_str() {
        "internal-maintenance" => Ok(ToolExecutionResult {
            summary: "Internal maintenance completed without external side effects.".to_string(),
        }),
        "praxis-data-write" => append_text(paths, manifest, request),
        "file-read" => read_file(paths, manifest, request),
        "shell-exec" => exec_shell_command(paths, &vault, manifest, request),
        "github-issues" => github_list_issues(paths, request),
        "github-prs" => github_list_prs(paths, request),
        "todo" => execute_todo_tool(paths, request),
        "clarify" => execute_clarify_tool(paths, request),
        "memory" => execute_memory_tool(paths, request),
        "cron" => execute_cron_tool(paths, request),
        "image" => crate::tools::image::execute_image_tool(paths, request),
        "vision" => execute_vision_tool(paths, request),
        "voice" => execute_voice_tool(paths, request),
        _ => match manifest.kind {
            ToolKind::Shell if manifest.path.as_deref().is_some_and(|p| !p.trim().is_empty()) => {
                run_shell(paths, &vault, manifest, request)
            }
            ToolKind::Http
                if manifest.endpoint.as_deref().is_some_and(|e| !e.trim().is_empty()) =>
            {
                run_http(paths, &vault, manifest, request)
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
    vault: &Vault,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let exe = manifest.path.as_deref().filter(|p| !p.trim().is_empty()).ok_or_else(|| {
        anyhow::anyhow!("shell tool {} has no 'path' field in its manifest", manifest.name)
    })?;

    let timeout_secs = manifest.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(1, 300);

    // Parse any extra arguments from the structured payload.
    let payload = parse_payload(request.payload_json.as_deref())?;
    let extra_args: Vec<String> = payload
        .params
        .get("args")
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .or_else(|| {
            payload
                .append_text
                .as_deref()
                .map(|text| text.split_whitespace().map(str::to_string).collect::<Vec<_>>())
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

    // Inject vault secrets as VAULT_<NAME> env vars, filtered by allowed_vault_keys if set.
    for (name, entry) in &vault.secrets {
        if manifest
            .allowed_vault_keys
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(name))
        {
            continue;
        }
        if let Some(value) = entry.resolve().ok().flatten() {
            let key = format!("VAULT_{}", name.to_ascii_uppercase().replace('-', "_"));
            cmd.env(key, value);
        }
    }

    // Inject OAuth tokens as env vars, filtered by allowed_oauth_providers if set.
    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if manifest
                .allowed_oauth_providers
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(provider))
            {
                continue;
            }
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
                bail!("shell tool {} timed out after {}s (pid {pid})", manifest.name, timeout_secs);
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    }

    let output = child
        .wait_with_output()
        .with_context(|| format!("failed to collect output of shell tool {}", manifest.name))?;

    let stdout_bytes = &output.stdout[..output.stdout.len().min(MAX_OUTPUT_BYTES)];
    let stderr_bytes = &output.stderr[..output.stderr.len().min(MAX_OUTPUT_BYTES)];
    let stdout = String::from_utf8_lossy(stdout_bytes).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr_bytes).trim().to_string();

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let detail = if stderr.is_empty() {
            stdout.chars().take(200).collect::<String>()
        } else {
            stderr.chars().take(200).collect::<String>()
        };
        bail!("shell tool {} exited with code {code}: {detail}", manifest.name);
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
    vault: &Vault,
    manifest: &ToolManifest,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let endpoint_template =
        manifest.endpoint.as_deref().filter(|e| !e.trim().is_empty()).ok_or_else(|| {
            anyhow::anyhow!("http tool {} has no 'endpoint' field in its manifest", manifest.name)
        })?;

    let method = manifest.method.as_deref().unwrap_or("GET").to_ascii_uppercase();

    let timeout_secs = manifest.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(1, 120);

    let payload = parse_payload(request.payload_json.as_deref())?;

    // Substitute {param} placeholders then $VAULT{name} references in the URL.
    let url = substitute_vault(&substitute_params(endpoint_template, &payload.params), vault);

    log::info!("executing http tool {} {} {}", manifest.name, method, url);

    if is_ssrf_blocked(&url) {
        bail!(
            "http tool {} targets a blocked internal/private address: {}",
            manifest.name,
            url.chars().take(80).collect::<String>()
        );
    }

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

    // Apply manifest headers, resolving $VAULT{name} references in values.
    for header_str in &manifest.headers {
        if let Some((key, value)) = header_str.split_once(':') {
            let resolved_value = substitute_vault(value.trim(), vault);
            builder = builder.header(key.trim(), resolved_value);
        }
    }

    // Inject OAuth tokens, filtered by allowed_oauth_providers if set.
    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if manifest
                .allowed_oauth_providers
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(provider))
            {
                continue;
            }
            if !token.is_expired() {
                let header_name = format!("X-Praxis-OAuth-{}", provider_title(provider));
                builder = builder.header(header_name, &token.access_token);
            }
        }
    }

    // Attach body: explicit payload.body beats manifest.body template.
    let body_str = payload
        .body
        .clone()
        .or_else(|| manifest.body.as_ref().map(|t| substitute_params(t, &payload.params)));

    if let Some(body) = body_str {
        builder = builder.header("Content-Type", "application/json").body(body);
    }

    let response = builder
        .send()
        .with_context(|| format!("failed to send request for http tool {}", manifest.name))?;

    let status = response.status();
    let body_text = response.text().unwrap_or_else(|_| "(unreadable body)".to_string());

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
        summary: format!("HTTP tool {} completed ({}).\n{}", manifest.name, status, snippet),
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

/// Block requests to private/internal networks to prevent SSRF attacks.
fn is_ssrf_blocked(url: &str) -> bool {
    // Extract the host portion from the URL for checking.
    let host = extract_host(url);
    let host = match host {
        Some(h) => h,
        None => return true, // Can't parse → block
    };

    // Block common internal hostnames.
    if host == "localhost" || host.ends_with(".local") || host.ends_with(".internal") {
        return true;
    }

    // Block IP-based private ranges.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                if v4.is_loopback()
                    || v4.is_private()
                    || v4.is_link_local()
                    || v4.is_broadcast()
                    || v4.is_documentation()
                {
                    return true;
                }
                // 169.254.169.254 — cloud metadata endpoint.
                if v4.octets() == [169, 254, 169, 254] {
                    return true;
                }
            }
            std::net::IpAddr::V6(v6) => {
                if v6.is_loopback() {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract the host portion from a URL string.
fn extract_host(url: &str) -> Option<String> {
    // Strip scheme.
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://"))?;
    // Strip credentials.
    let rest = if let Some(at_pos) = rest.find('@') {
        &rest[at_pos + 1..]
    } else {
        rest
    };
    // Strip port and path.
    let host_port = rest.split('/').next()?;
    let host = host_port.split(':').next()?;
    Some(host.to_string())
}

/// Substitute `$VAULT{name}` references in `template` with resolved vault values.
/// Unresolved references are left as-is so they surface as obvious errors.
fn substitute_vault(template: &str, vault: &Vault) -> String {
    let mut result = template.to_string();
    // Iterate over known secret names and substitute any $VAULT{name} occurrences.
    for name in vault.secrets.keys() {
        let placeholder = format!("$VAULT{{{name}}}");
        if result.contains(&placeholder)
            && let Some(value) = vault.resolve_optional(name)
        {
            result = result.replace(&placeholder, &value);
        }
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
            anyhow::anyhow!("file-read path {} is not within any allowed_read_paths root", raw_path)
        })?
    };

    if let Ok(meta) = fs::symlink_metadata(&full_path)
        && meta.file_type().is_symlink()
    {
        anyhow::bail!("file-read refuses to follow symlink at {}", full_path.display());
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

/// Validate that a shell command does not contain characters that enable
/// command injection when passed to `/bin/bash -c`. Returns `Err` if any
/// dangerous metacharacters are found.
fn validate_shell_command(command: &str) -> Result<()> {
    if command.is_empty() {
        bail!("shell-exec command must not be empty");
    }
    if let Some(ch) = command.chars().find(|c| DANGEROUS_SHELL_CHARS.contains(c)) {
        bail!(
            "shell-exec command contains dangerous metacharacter '{}'. \
             For complex operations, create a dedicated shell tool manifest.",
            ch
        );
    }
    Ok(())
}

fn exec_shell_command(
    paths: &PraxisPaths,
    vault: &Vault,
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

    validate_shell_command(command)?;

    let timeout_secs = manifest.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(1, 300);

    log::info!(
        "executing shell-exec (timeout {}s): {}",
        timeout_secs,
        &command[..command.len().min(80)]
    );

    let mut cmd = Command::new("/bin/bash");
    cmd.args(["-c", command]);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Inject vault secrets as VAULT_<NAME> env vars, filtered by allowed_vault_keys if set.
    for (name, entry) in &vault.secrets {
        if manifest
            .allowed_vault_keys
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(name))
        {
            continue;
        }
        if let Some(value) = entry.resolve().ok().flatten() {
            let key = format!("VAULT_{}", name.to_ascii_uppercase().replace('-', "_"));
            cmd.env(key, value);
        }
    }

    // Inject OAuth tokens as env vars, filtered by allowed_oauth_providers if set.
    let oauth_store = OAuthTokenStore::new(&paths.data_dir);
    if let Ok(tokens) = oauth_store.load() {
        for (provider, token) in &tokens {
            if manifest
                .allowed_oauth_providers
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(provider))
            {
                continue;
            }
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

    let output = child.wait_with_output().context("failed to collect shell-exec output")?;

    let stdout_bytes = &output.stdout[..output.stdout.len().min(MAX_OUTPUT_BYTES)];
    let stderr_bytes = &output.stderr[..output.stderr.len().min(MAX_OUTPUT_BYTES)];
    let stdout = String::from_utf8_lossy(stdout_bytes).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr_bytes).trim().to_string();

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
    let Some(rel_path) = payload.params.get("path") else {
        return Ok(fallback_result(
            manifest,
            request,
            "Approved request did not specify a path, so Praxis kept this run safe and skipped the file mutation.",
        ));
    };

    let relative = normalize_relative(rel_path)?;
    let full_path = paths.data_dir.join(&relative);
    if !full_path.starts_with(&paths.data_dir) {
        anyhow::bail!("praxis-data-write path escapes the Praxis data directory");
    }

    // Symlink safety.
    if let Ok(meta) = fs::symlink_metadata(&full_path)
        && meta.file_type().is_symlink()
    {
        anyhow::bail!("praxis-data-write refuses to follow symlink at {}", full_path.display());
    }

    let text = match (
        payload.append_text.as_deref(),
        payload.body.as_deref(),
    ) {
        (Some(t), _) => t.to_string(),
        (_, Some(b)) => b.to_string(),
        _ => {
            return Ok(fallback_result(
                manifest,
                request,
                "Approved request contained no text payload, so Praxis kept this run safe and skipped the file mutation.",
            ));
        }
    };

    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent for {}", full_path.display()))?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&full_path)
        .with_context(|| format!("failed to open {} for append", full_path.display()))?;

    file.write_all(text.as_bytes())
        .with_context(|| format!("failed to write to {}", full_path.display()))?;
    file.write_all(b"\n")?;
    file.flush()?;

    Ok(ToolExecutionResult {
        summary: format!("Wrote {} bytes to {}.", text.len(), relative.display()),
    })
}

fn execute_todo_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let action = payload
        .params
        .get("action")
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("todo requires 'action' parameter"))?;
    let id = payload.params.get("id").map(|s| s.as_str());
    let content = payload.params.get("content").map(|s| s.as_str());
    let status = payload.params.get("status").map(|s| s.as_str());

    let summary = execute_todo_action(&paths.todo_file, action, id, content, status)?;
    Ok(ToolExecutionResult { summary })
}

fn execute_clarify_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let question = payload
        .params
        .get("question")
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("clarify requires 'question' parameter"))?;
    let choices: Vec<String> = if let Some(raw) = payload.params.get("choices") {
        raw.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };

    let summary = execute_clarify(&paths.bus_file, question, &choices)?;
    Ok(ToolExecutionResult { summary })
}

fn execute_memory_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let action = payload
        .params
        .get("action")
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("memory requires action parameter"))?;
    let key = payload.params.get("key").map(|s| s.as_str());
    let value = payload.params.get("value").map(|s| s.as_str());
    let tags: Vec<String> = if let Some(raw) = payload.params.get("tags") {
        raw.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };

    use crate::memory::UserMemory;
    let mut memory = UserMemory::load(&paths.user_memory_file).unwrap_or_default();

    let summary = match action {
        "upsert" => {
            let k = key.ok_or_else(|| anyhow::anyhow!("memory upsert requires 'key'"))?;
            let v = value.ok_or_else(|| anyhow::anyhow!("memory upsert requires 'value'"))?;
            memory.upsert(k, v, tags)?;
            format!("Memory entry '{}' saved.", k)
        }
        "search" => {
            let q = key.ok_or_else(|| anyhow::anyhow!("memory search requires 'key' as query"))?;
            let results = memory.search(q);
            if results.is_empty() {
                "No memory entries matched.".to_string()
            } else {
                results
                    .into_iter()
                    .map(|e| format!("- {}: {}", e.key, e.value))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "forget" => {
            let k = key.ok_or_else(|| anyhow::anyhow!("memory forget requires 'key'"))?;
            let removed = memory.forget(k);
            if removed {
                format!("Memory entry '{}' removed.", k)
            } else {
                format!("Memory entry '{}' not found.", k)
            }
        }
        "list" => {
            let keys = memory.keys();
            if keys.is_empty() {
                "No memory entries.".to_string()
            } else {
                keys.iter()
                    .map(|k| format!("- {}", k))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        other => anyhow::bail!("unknown memory action: {other}"),
    };

    Ok(ToolExecutionResult { summary })
}

fn execute_cron_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    use super::cron::{ScheduledJobs, create_job};

    let payload = parse_payload(request.payload_json.as_deref())?;
    let action = payload
        .params
        .get("action")
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("cron requires 'action' parameter"))?;

    let mut jobs = ScheduledJobs::load(&paths.scheduled_jobs_file).unwrap_or_default();

    let summary = match action {
        "create" => {
            let name = payload
                .params
                .get("name")
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unnamed".to_string());
            let schedule = payload
                .params
                .get("schedule")
                .ok_or_else(|| anyhow::anyhow!("cron create requires 'schedule'"))?
                .to_string();
            let task = payload
                .params
                .get("task")
                .ok_or_else(|| anyhow::anyhow!("cron create requires 'task'"))?
                .to_string();
            let job = create_job(name, schedule, task)?;
            jobs.add(job.clone());
            format!("Created cron job '{}'.", job.id)
        }
        "list" => {
            if jobs.jobs.is_empty() {
                "No scheduled jobs.".to_string()
            } else {
                jobs.jobs
                    .iter()
                    .map(|j| format!("- [{}] {} ({})", j.id, j.name, j.schedule))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "remove" => {
            let id = payload
                .params
                .get("id")
                .ok_or_else(|| anyhow::anyhow!("cron remove requires 'id'"))?;
            let removed = jobs.remove(id);
            if removed {
                format!("Removed cron job '{id}'.")
            } else {
                format!("Cron job '{id}' not found.")
            }
        }
        other => anyhow::bail!("unknown cron action: {other}"),
    };

    jobs.save(&paths.scheduled_jobs_file)?;
    Ok(ToolExecutionResult { summary })
}

fn execute_vision_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    
    let image_url = payload.params.get("image_url").map(|s| s.to_string());
    let image_path = payload.params.get("image_path").map(|s| s.to_string());
    let question = payload.params.get("question").map(|s| s.to_string());
    let detail = payload.params.get("detail").map(|s| s.to_string());

    if image_url.is_none() && image_path.is_none() {
        anyhow::bail!("vision requires either 'image_url' or 'image_path' parameter");
    }

    let vision_tool = VisionTool::new();
    let params = VisionParameters {
        image_url,
        image_path,
        question,
        detail,
    };

    let input_content = vision_tool.execute(&params, paths)?;
    
    // Convert InputContent to a string representation for the summary
    let summary = match input_content {
        crate::backend::InputContent::Text(text) => text,
        crate::backend::InputContent::Blocks(blocks) => {
            let text_parts: Vec<String> = blocks
                .into_iter()
                .map(|block| match block {
                    crate::backend::ContentBlock::Text { text } => text,
                    crate::backend::ContentBlock::ImageUrl { image_url } => {
                        format!("[Image: {}]", image_url.url)
                    }
                })
                .collect();
            text_parts.join("\n")
        }
    };

    Ok(ToolExecutionResult {
        summary: format!("Vision tool prepared content blocks for analysis.\n{}", summary),
    })
}

fn execute_voice_tool(
    paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    use super::voice::{VoiceTool, VoiceParameters};

    let payload = parse_payload(request.payload_json.as_deref())?;
    
    let action = payload.params.get("action").map(|s| s.to_string());
    let text = payload.params.get("text").map(|s| s.to_string());
    let audio_path = payload.params.get("audio_path").map(|s| s.to_string());
    let voice = payload.params.get("voice").map(|s| s.to_string());
    let language = payload.params.get("language").map(|s| s.to_string());

    let voice_tool = VoiceTool::new();
    let params = VoiceParameters {
        action,
        text,
        audio_path,
        voice,
        language,
    };

    let summary = voice_tool.execute(&params, paths)?;
    
    Ok(ToolExecutionResult {
        summary: format!("Voice tool executed.\n{}", summary),
    })
}

fn github_list_issues(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let Some(token) = OAuthTokenStore::new(&_paths.data_dir)
        .get("github")
        .ok()
        .flatten()
        .filter(|t| !t.is_expired())
    else {
        return Ok(ToolExecutionResult {
            summary: "No valid GitHub OAuth token. Run `praxis oauth login github`.".to_string(),
        });
    };

    let payload = parse_payload(request.payload_json.as_deref())?;
    let repo = payload
        .params
        .get("repo")
        .map(|s| s.as_str())
        .unwrap_or("coe0718/axonix");
    let state = payload
        .params
        .get("state")
        .map(|s| s.as_str())
        .unwrap_or("open");
    let limit: u32 = payload
        .params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let url = format!("https://api.github.com/repos/{repo}/issues?state={state}&per_page={limit}");
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client
        .get(&url)
        .bearer_auth(&token.access_token)
        .header("User-Agent", "praxis-agent")
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("GitHub API returned {status}: {}", body.chars().take(200).collect::<String>());
    }

    let issues: Vec<serde_json::Value> = response.json()?;
    let lines: Vec<String> = issues
        .iter()
        .filter_map(|issue| {
            let number = issue.get("number")?.as_u64()?;
            let title = issue.get("title")?.as_str()?;
            Some(format!("#{number}: {title}"))
        })
        .collect();

    Ok(ToolExecutionResult {
        summary: if lines.is_empty() {
            "No issues found.".to_string()
        } else {
            lines.join("\n")
        },
    })
}

fn github_list_prs(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<ToolExecutionResult> {
    let Some(token) = OAuthTokenStore::new(&_paths.data_dir)
        .get("github")
        .ok()
        .flatten()
        .filter(|t| !t.is_expired())
    else {
        return Ok(ToolExecutionResult {
            summary: "No valid GitHub OAuth token. Run `praxis oauth login github`.".to_string(),
        });
    };

    let payload = parse_payload(request.payload_json.as_deref())?;
    let repo = payload
        .params
        .get("repo")
        .map(|s| s.as_str())
        .unwrap_or("coe0718/axonix");
    let state = payload
        .params
        .get("state")
        .map(|s| s.as_str())
        .unwrap_or("open");
    let limit: u32 = payload
        .params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let url = format!("https://api.github.com/repos/{repo}/pulls?state={state}&per_page={limit}");
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client
        .get(&url)
        .bearer_auth(&token.access_token)
        .header("User-Agent", "praxis-agent")
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("GitHub API returned {status}: {}", body.chars().take(200).collect::<String>());
    }

    let prs: Vec<serde_json::Value> = response.json()?;
    let lines: Vec<String> = prs
        .iter()
        .filter_map(|pr| {
            let number = pr.get("number")?.as_u64()?;
            let title = pr.get("title")?.as_str()?;
            Some(format!("#{number}: {title}"))
        })
        .collect();

    Ok(ToolExecutionResult {
        summary: if lines.is_empty() {
            "No pull requests found.".to_string()
        } else {
            lines.join("\n")
        },
    })
}

fn fallback_result(manifest: &ToolManifest, request: &StoredApprovalRequest, message: &str) -> ToolExecutionResult {
    let payload = parse_payload(request.payload_json.as_deref()).ok();
    let params_json = payload
        .map(|p| serde_json::to_string_pretty(&p.params).unwrap_or_default())
        .unwrap_or_default();

    ToolExecutionResult {
        summary: format!(
            "Tool: {}\nStatus: {}\nMessage: {}\nParams: {}",
            manifest.name, "no-adapter", message, params_json
        ),
    }
}
