//! Hook system — shell scripts that run in response to runtime events.
//!
//! Hooks let operators customise Praxis behaviour without modifying source
//! code.  They are declared in `hooks.toml` and loaded at runtime.
//!
//! ## Hook kinds
//!
//! | Kind          | Blocking? | Effect on event                              |
//! |---------------|-----------|----------------------------------------------|
//! | `observer`    | No        | Fire-and-forget side effect (notify, log …)  |
//! | `interceptor` | Yes       | Non-zero exit **aborts** the event           |
//! | `approval`    | Yes       | Stdout `approve`/`reject`/`defer` decides approval |
//!
//! ## `hooks.toml` format
//!
//! ```toml
//! [[hooks]]
//! event   = "session.end"
//! kind    = "observer"
//! script  = "/home/user/notify.sh"
//!
//! [[hooks]]
//! event   = "phase.act.start"
//! kind    = "interceptor"
//! script  = "/home/user/gate.sh"
//! # filter matches tool_name or phase name via glob (optional)
//! filter  = "*"
//! timeout_secs = 10
//!
//! [[hooks]]
//! event   = "approval.before"
//! kind    = "approval"
//! script  = "/home/user/auto-approve.sh"
//! filter  = "safe-read-*"   # glob on tool_name
//! ```
//!
//! ## Context passed to every hook
//!
//! All hooks receive the following environment variables:
//!
//! | Variable                | Value                               |
//! |-------------------------|-------------------------------------|
//! | `PRAXIS_EVENT`          | Event name (e.g. `session.end`)     |
//! | `PRAXIS_DATA_DIR`       | Absolute path to the data directory |
//! | `PRAXIS_SESSION_ID`     | Current session DB id (if known)    |
//! | `PRAXIS_PHASE`          | Current phase name (if applicable)  |
//! | `PRAXIS_TOOL_NAME`      | Tool name (tool hooks only)         |
//! | `PRAXIS_TOOL_REQUEST_ID`| Approval request id (tool hooks)    |
//! | `PRAXIS_OUTCOME`        | Session outcome (session.end only)  |

use std::{
    collections::HashMap,
    fs,
    io::Read as _,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookKind {
    /// Fire-and-forget; return code is ignored.
    Observer,
    /// Blocking; non-zero exit aborts the event.
    Interceptor,
    /// Blocking; reads `approve` / `reject` / `defer` from stdout.
    Approval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    /// Event name to listen for.  Supports simple `*` glob suffix
    /// (e.g. `phase.*` matches all phase events).
    pub event: String,
    pub kind: HookKind,
    /// Absolute path to the script to execute.
    pub script: PathBuf,
    /// Optional secondary glob filter (matches tool name, phase name, etc.).
    /// Empty / absent = match all.
    #[serde(default)]
    pub filter: Option<String>,
    /// Maximum seconds to wait for the script.  Defaults to 10.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    10
}

impl HookEntry {
    /// True if this hook matches the given event name.
    fn matches_event(&self, event: &str) -> bool {
        glob_match(&self.event, event)
    }

    /// True if the secondary filter matches `value` (or no filter is set).
    fn matches_filter(&self, value: &str) -> bool {
        match &self.filter {
            None => true,
            Some(f) if f.is_empty() || f == "*" => true,
            Some(f) => glob_match(f, value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HookConfig {
    #[serde(default)]
    pub hooks: Vec<HookEntry>,
}

/// Context values passed to every hook as environment variables.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    pub event: String,
    pub data_dir: PathBuf,
    pub session_id: Option<i64>,
    pub phase: Option<String>,
    pub tool_name: Option<String>,
    pub tool_request_id: Option<i64>,
    pub outcome: Option<String>,
    /// Extra key→value pairs for future extensibility.
    pub extra: HashMap<String, String>,
}

impl HookContext {
    pub fn new(event: impl Into<String>, data_dir: PathBuf) -> Self {
        Self {
            event: event.into(),
            data_dir,
            ..Default::default()
        }
    }

    pub fn with_session(mut self, id: i64) -> Self {
        self.session_id = Some(id);
        self
    }
    pub fn with_phase(mut self, p: impl Into<String>) -> Self {
        self.phase = Some(p.into());
        self
    }
    pub fn with_tool(mut self, name: impl Into<String>, id: Option<i64>) -> Self {
        self.tool_name = Some(name.into());
        self.tool_request_id = id;
        self
    }
    pub fn with_outcome(mut self, o: impl Into<String>) -> Self {
        self.outcome = Some(o.into());
        self
    }

    fn to_env(&self) -> Vec<(String, String)> {
        let mut vars = vec![
            ("PRAXIS_EVENT".into(), self.event.clone()),
            ("PRAXIS_DATA_DIR".into(), self.data_dir.display().to_string()),
        ];
        if let Some(id) = self.session_id {
            vars.push(("PRAXIS_SESSION_ID".into(), id.to_string()));
        }
        if let Some(phase) = &self.phase {
            vars.push(("PRAXIS_PHASE".into(), phase.clone()));
        }
        if let Some(tool) = &self.tool_name {
            vars.push(("PRAXIS_TOOL_NAME".into(), tool.clone()));
        }
        if let Some(rid) = self.tool_request_id {
            vars.push(("PRAXIS_TOOL_REQUEST_ID".into(), rid.to_string()));
        }
        if let Some(outcome) = &self.outcome {
            vars.push(("PRAXIS_OUTCOME".into(), outcome.clone()));
        }
        for (k, v) in &self.extra {
            vars.push((k.clone(), v.clone()));
        }
        vars
    }
}

/// Verdict returned by an approval hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalVerdict {
    /// Auto-approve — skip the operator queue.
    Approve,
    /// Auto-reject with an explanatory note.
    Reject(String),
    /// Pass through to the normal approval queue.
    Defer,
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Loads and executes registered hooks.
#[derive(Debug, Clone, Default)]
pub struct HookRunner {
    hooks: Vec<HookEntry>,
}

impl HookRunner {
    /// Load hooks from `hooks.toml`.  Returns an empty runner when the file
    /// does not exist.
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let config: HookConfig = toml::from_str(&raw)
                    .with_context(|| format!("invalid TOML in hooks file {}", path.display()))?;
                Ok(Self { hooks: config.hooks })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn from_paths(paths: &PraxisPaths) -> Self {
        Self::load(&paths.hooks_file).unwrap_or_default()
    }

    /// True if any hooks are registered.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Fire all observer hooks matching `event` / filter.
    ///
    /// Observers are spawned as background processes; errors are logged but
    /// never propagated — they must not block the agent.
    pub fn fire_observer(&self, event: &str, ctx: &HookContext, filter: &str) {
        for hook in self.matching(event, HookKind::Observer, filter) {
            if !validate_hook_script(&hook.script) {
                continue;
            }
            let env = ctx.to_env();
            let script = hook.script.clone();
            let timeout = Duration::from_secs(hook.timeout_secs);
            match Command::new(&script)
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    std::thread::spawn(move || {
                        let _ = wait_with_timeout(&mut child, timeout);
                    });
                }
                Err(e) => log::warn!("hooks: observer '{}' failed to spawn: {e}", script.display()),
            }
        }
    }

    /// Run all interceptor hooks matching `event` / filter.
    ///
    /// Returns `Err` (aborting the event) if any interceptor exits non-zero.
    pub fn fire_interceptor(&self, event: &str, ctx: &HookContext, filter: &str) -> Result<()> {
        for hook in self.matching(event, HookKind::Interceptor, filter) {
            if !validate_hook_script(&hook.script) {
                continue;
            }
            let env = ctx.to_env();
            let timeout = Duration::from_secs(hook.timeout_secs);
            let mut child = Command::new(&hook.script)
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .with_context(|| {
                    format!("failed to spawn interceptor '{}'", hook.script.display())
                })?;
            let status = wait_with_timeout(&mut child, timeout)
                .with_context(|| format!("interceptor '{}' timed out", hook.script.display()))?;
            if !status.success() {
                anyhow::bail!(
                    "hook interceptor '{}' blocked event '{event}' (exit {:?})",
                    hook.script.display(),
                    status.code()
                );
            }
        }
        Ok(())
    }

    /// Run approval hooks for the given tool name.
    ///
    /// Returns `Defer` if no approval hooks are registered for this tool,
    /// `Approve` or `Reject` if a hook decides.  The first decisive verdict
    /// wins; remaining hooks are skipped.
    pub fn fire_approval_hooks(
        &self,
        tool_name: &str,
        ctx: &HookContext,
        request_json: Option<&str>,
    ) -> ApprovalVerdict {
        for hook in self.matching("approval.before", HookKind::Approval, tool_name) {
            if !validate_hook_script(&hook.script) {
                continue;
            }
            let mut env = ctx.to_env();
            if let Some(json) = request_json {
                env.push(("PRAXIS_APPROVAL_JSON".into(), json.to_string()));
            }
            let timeout = Duration::from_secs(hook.timeout_secs);
            let mut child = match Command::new(&hook.script)
                .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("approval hook '{}' spawn failed: {e}", hook.script.display());
                    continue;
                }
            };
            // Drain stdout in a background thread so the pipe never blocks the
            // process wait, then enforce the timeout on the process itself.
            let mut stdout_handle = child.stdout.take().expect("stdout is piped");
            let stdout_thread = std::thread::spawn(move || -> String {
                let mut buf = String::new();
                let _ = stdout_handle.read_to_string(&mut buf);
                buf
            });
            if let Err(e) = wait_with_timeout(&mut child, timeout) {
                log::warn!("approval hook '{}' timed out: {e}", hook.script.display());
                let _ = stdout_thread.join();
                continue;
            }
            let raw = stdout_thread.join().unwrap_or_default();
            let stdout = raw.trim().to_lowercase();
            match stdout.as_str() {
                "approve" => {
                    log::info!(
                        "hooks: approval hook '{}' auto-approved tool '{tool_name}'",
                        hook.script.display()
                    );
                    return ApprovalVerdict::Approve;
                }
                s if s.starts_with("reject") => {
                    let note = s.strip_prefix("reject").unwrap_or("").trim().to_string();
                    log::info!(
                        "hooks: approval hook '{}' rejected tool '{tool_name}': {note}",
                        hook.script.display()
                    );
                    return ApprovalVerdict::Reject(note.to_string());
                }
                _ => {
                    // "defer" or anything else — continue to next hook.
                }
            }
        }
        ApprovalVerdict::Defer
    }

    fn matching<'a>(
        &'a self,
        event: &str,
        kind: HookKind,
        filter: &str,
    ) -> impl Iterator<Item = &'a HookEntry> {
        self.hooks
            .iter()
            .filter(move |h| h.kind == kind && h.matches_event(event) && h.matches_filter(filter))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_hook_script(script: &Path) -> bool {
    if !script.is_absolute() {
        log::warn!("hooks: script '{}' must be an absolute path — skipping", script.display());
        return false;
    }
    if script.is_symlink() {
        log::warn!("hooks: script '{}' is a symlink — skipping for security", script.display());
        return false;
    }
    true
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    anyhow::bail!("hook timed out after {}s", timeout.as_secs());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == value;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return value.ends_with(suffix);
    }
    // Internal wildcard — split and match.
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remaining = value;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 {
            return remaining.ends_with(part);
        } else {
            match remaining.find(part) {
                Some(pos) => remaining = &remaining[pos + part.len()..],
                None => return false,
            }
        }
    }
    true
}

// ── CLI helpers ───────────────────────────────────────────────────────────────

/// Install a hook entry into `hooks.toml`.
pub fn install_hook(paths: &PraxisPaths, entry: HookEntry) -> Result<()> {
    let mut config = load_config(&paths.hooks_file)?;
    config.hooks.push(entry);
    save_config(&paths.hooks_file, &config)
}

/// Remove all hooks whose script path matches `script`.
pub fn remove_hook(paths: &PraxisPaths, script: &Path) -> Result<usize> {
    let mut config = load_config(&paths.hooks_file)?;
    let before = config.hooks.len();
    config.hooks.retain(|h| h.script != script);
    let removed = before - config.hooks.len();
    if removed > 0 {
        save_config(&paths.hooks_file, &config)?;
    }
    Ok(removed)
}

fn load_config(path: &Path) -> Result<HookConfig> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            toml::from_str(&raw).with_context(|| format!("invalid hooks config {}", path.display()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HookConfig::default()),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn save_config(path: &Path, config: &HookConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = toml::to_string_pretty(config).context("failed to serialise hook config")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_exact_match() {
        assert!(glob_match("session.end", "session.end"));
        assert!(!glob_match("session.end", "session.start"));
    }

    #[test]
    fn glob_star_suffix() {
        assert!(glob_match("phase.*", "phase.orient.start"));
        assert!(glob_match("phase.*", "phase.reflect.end"));
        assert!(!glob_match("phase.*", "session.end"));
    }

    #[test]
    fn glob_star_only() {
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn empty_runner_fires_nothing() {
        let runner = HookRunner::default();
        let ctx = HookContext::new("session.end", "/tmp".into());
        // Should not panic.
        runner.fire_observer("session.end", &ctx, "*");
        runner.fire_interceptor("phase.act.start", &ctx, "*").unwrap();
        assert_eq!(runner.fire_approval_hooks("any-tool", &ctx, None), ApprovalVerdict::Defer);
    }
}
