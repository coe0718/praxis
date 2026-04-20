//! System anomaly correlation — captures lightweight system metrics
//! (CPU load, memory usage, disk usage) alongside session outcomes so the
//! operator can spot correlations between resource pressure and degraded
//! agent performance.
//!
//! Snapshots are appended to `system_anomalies.jsonl` during maintenance and
//! at the start of Reflect when a session had a bad outcome.  The last N
//! snapshots are exposed in `praxis status` and the brief.

use std::{
    fs::{self, OpenOptions},
    io::Write as IoWrite,
    path::Path,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum number of anomaly records retained in the JSONL file.
const MAX_RECORDS: usize = 200;

/// A point-in-time system resource snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub recorded_at: DateTime<Utc>,
    /// Load average (1-minute) — `None` on platforms without `/proc/loadavg`.
    pub load_avg_1m: Option<f64>,
    /// Approximate RSS of the current process in MB.
    pub process_rss_mb: Option<u64>,
    /// Disk usage of the data directory in MB.
    pub data_dir_mb: Option<u64>,
    /// Session outcome that triggered this snapshot (`None` for routine captures).
    pub session_outcome: Option<String>,
    /// True when any metric crossed a warning threshold.
    pub is_anomaly: bool,
}

impl SystemSnapshot {
    /// Capture current metrics, optionally tagging with a session outcome.
    pub fn capture(data_dir: &Path, session_outcome: Option<String>) -> Self {
        let load_avg_1m = read_load_avg();
        let process_rss_mb = read_process_rss_mb();
        let data_dir_mb = dir_size_mb(data_dir);

        let is_anomaly = load_avg_1m.is_some_and(|l| l > 4.0)
            || process_rss_mb.is_some_and(|r| r > 512)
            || session_outcome
                .as_deref()
                .is_some_and(|o| o.contains("fail") || o.contains("error"));

        Self {
            recorded_at: Utc::now(),
            load_avg_1m,
            process_rss_mb,
            data_dir_mb,
            session_outcome,
            is_anomaly,
        }
    }
}

/// Append a snapshot to the JSONL log and prune if over the retention limit.
pub fn record_snapshot(path: &Path, snapshot: &SystemSnapshot) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let line = serde_json::to_string(snapshot).context("failed to serialize snapshot")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("failed to write {}", path.display()))?;
    prune_log(path)?;
    Ok(())
}

/// Load the last `limit` snapshots from the JSONL log.
pub fn load_recent(path: &Path, limit: usize) -> Result<Vec<SystemSnapshot>> {
    let raw = match fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("failed to read {}", path.display())),
    };
    let records: Vec<SystemSnapshot> = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    Ok(records.into_iter().rev().take(limit).collect())
}

/// Count how many of the recent snapshots are anomalies.
pub fn recent_anomaly_count(path: &Path, limit: usize) -> usize {
    load_recent(path, limit)
        .unwrap_or_default()
        .iter()
        .filter(|s| s.is_anomaly)
        .count()
}

// ── Platform helpers ──────────────────────────────────────────────────────────

fn read_load_avg() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let raw = fs::read_to_string("/proc/loadavg").ok()?;
        raw.split_whitespace().next()?.parse().ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn read_process_rss_mb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let raw = fs::read_to_string("/proc/self/status").ok()?;
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb / 1024);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn dir_size_mb(path: &Path) -> Option<u64> {
    let mut total = 0u64;
    for entry in fs::read_dir(path).ok()?.flatten() {
        if let Ok(meta) = entry.metadata()
            && meta.is_file()
        {
            total += meta.len();
        }
    }
    Some(total / (1024 * 1024))
}

/// Keep at most `MAX_RECORDS` lines in the JSONL file.
fn prune_log(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() <= MAX_RECORDS {
        return Ok(());
    }
    let kept = &lines[lines.len() - MAX_RECORDS..];
    let mut content = kept.join("\n");
    content.push('\n');
    fs::write(path, content).with_context(|| format!("failed to prune {}", path.display()))
}
