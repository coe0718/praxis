//! Cron extensions — no_agent script mode, wake_gate, per-job workdir.
//!
//! #5 Cron extensions (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Extends the existing `src/tools/cron.rs` with:
//! - `no_agent`: run shell scripts directly without LLM involvement
//! - `wake_gate`: script output can signal `wakeAgent=false` to skip agent run
//! - `workdir` per-job: already in ScheduledJob, this adds daemon-side enforcement
//!
//! The daemon's cron loop calls `process_due_jobs()` which checks these features.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::tools::cron::{ScheduledJob, ScheduledJobs};

/// Extended job processing options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronExtensions {
    /// When true, the job's script is run directly and its stdout becomes
    /// the delivered message — no LLM session is triggered.
    #[serde(default)]
    pub no_agent: bool,
    /// When true, the daemon checks the script's stdout for a JSON line
    /// containing `"wakeAgent": false` — if found, the agent session is skipped
    /// but the output is still delivered.
    #[serde(default)]
    pub wake_gate: bool,
}

/// Process a script-mode (no_agent) job.
///
/// Runs the script, captures stdout, and returns it as the message payload.
/// If `wake_gate` is enabled, checks output for `wakeAgent: false`.
pub fn run_script_job(
    job: &ScheduledJob,
    script: &str,
    extensions: &CronExtensions,
) -> Result<ScriptJobResult> {
    let workdir = job.workdir.as_deref().unwrap_or(".");

    let output = Command::new("/bin/bash")
        .arg("-c")
        .arg(script)
        .current_dir(workdir)
        .output()
        .with_context(|| format!("run script for job {}", job.id))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        log::warn!(
            "cron script job {} failed: exit={}, stderr={}",
            job.id,
            output.status.code().unwrap_or(-1),
            &stderr[..stderr.len().min(200)]
        );
    }

    // Check wake gate
    let should_wake_agent = if extensions.wake_gate {
        !stdout.contains("\"wakeAgent\": false") && !stdout.contains("\"wakeAgent\":false")
    } else {
        !extensions.no_agent
    };

    // Clean output: strip wake gate markers before delivery
    let clean_output = if extensions.wake_gate {
        stdout
            .lines()
            .filter(|l| !l.contains("wakeAgent"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        stdout
    };

    Ok(ScriptJobResult {
        output: clean_output,
        should_wake_agent,
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// Result of running a script-mode cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptJobResult {
    /// The script's stdout (cleaned of wake_gate markers).
    pub output: String,
    /// Whether the daemon should trigger an agent session.
    pub should_wake_agent: bool,
    /// The script's exit code.
    pub exit_code: i32,
}

/// Check if any jobs are due and process them.
///
/// This is called from the daemon's cron loop. For `no_agent` jobs,
/// the script runs directly. For regular jobs, a WakeIntent is written.
pub fn process_due_jobs(
    jobs_path: &Path,
    extensions_fn: impl Fn(&ScheduledJob) -> CronExtensions,
    script_fn: impl Fn(&ScheduledJob) -> Option<String>,
) -> Result<Vec<ScriptJobResult>> {
    let raw =
        fs::read_to_string(jobs_path).with_context(|| format!("read {}", jobs_path.display()))?;
    let store: ScheduledJobs =
        serde_json::from_str(&raw).with_context(|| format!("parse {}", jobs_path.display()))?;

    let mut results = Vec::new();

    for job in &store.jobs {
        let ext = extensions_fn(job);
        if ext.no_agent
            && let Some(script) = script_fn(job) {
                match run_script_job(job, &script, &ext) {
                    Ok(result) => {
                        log::info!(
                            "cron: no_agent job {} completed (exit={}, wake={})",
                            job.id,
                            result.exit_code,
                            result.should_wake_agent
                        );
                        results.push(result);
                    }
                    Err(e) => {
                        log::warn!("cron: no_agent job {} failed: {:#}", job.id, e);
                        results.push(ScriptJobResult {
                            output: format!("error: {:#}", e),
                            should_wake_agent: false,
                            exit_code: -1,
                        });
                    }
                }
            }
        // Regular jobs are handled by the existing daemon WakeIntent flow
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_gate_detection() {
        let ext = CronExtensions {
            no_agent: true,
            wake_gate: true,
        };
        let job = ScheduledJob {
            id: "test".to_string(),
            name: "test".to_string(),
            schedule: "daily".to_string(),
            kind: crate::tools::cron::ScheduleKind::Preset {
                preset: crate::tools::cron::SchedulePreset::Daily,
            },
            task: "test".to_string(),
            created_at: chrono::Utc::now(),
            last_fired_at: None,
            next_fire_at: chrono::Utc::now(),
            recurring: true,
            fire_count: 0,
            workdir: None,
            context_from: None,
            no_agent: false,
            wake_gate: false,
        };

        let result = run_script_job(&job, "echo '{\"wakeAgent\": false}'", &ext).unwrap();
        assert!(!result.should_wake_agent);
    }

    #[test]
    fn default_extensions() {
        let ext = CronExtensions::default();
        assert!(!ext.no_agent);
        assert!(!ext.wake_gate);
    }
}
