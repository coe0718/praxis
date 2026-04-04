use std::{fs, path::Path, sync::OnceLock, time::Instant};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::time::Clock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeartbeatRecord {
    pub component: String,
    pub phase: String,
    pub detail: String,
    pub updated_at: String,
    pub updated_at_unix_ms: i64,
    pub pid: u32,
    pub process_uptime_ms: u64,
}

pub fn write_heartbeat(
    path: &Path,
    component: &str,
    phase: &str,
    detail: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let record = HeartbeatRecord {
        component: component.to_string(),
        phase: phase.to_string(),
        detail: detail.to_string(),
        updated_at: now.to_rfc3339(),
        updated_at_unix_ms: now.timestamp_millis(),
        pid: std::process::id(),
        process_uptime_ms: START.get_or_init(Instant::now).elapsed().as_millis() as u64,
    };
    fs::write(
        path,
        serde_json::to_string_pretty(&record).context("failed to serialize heartbeat")?,
    )
    .with_context(|| format!("failed to write {}", path.display()))
}

pub fn read_heartbeat(path: &Path) -> Result<HeartbeatRecord> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("invalid heartbeat JSON in {}", path.display()))
}

pub fn check_heartbeat<C: Clock>(
    clock: &C,
    path: &Path,
    max_age_seconds: i64,
) -> Result<HeartbeatRecord> {
    if max_age_seconds <= 0 {
        bail!("max_age_seconds must be greater than 0");
    }
    let record = read_heartbeat(path)?;
    let updated_at = DateTime::parse_from_rfc3339(&record.updated_at)
        .context("failed to parse heartbeat timestamp")?
        .with_timezone(&Utc);
    let age = clock.now_utc() - updated_at;
    if age.num_seconds() > max_age_seconds {
        bail!(
            "heartbeat is stale: age={}s max={}s phase={}",
            age.num_seconds(),
            max_age_seconds,
            record.phase
        );
    }
    Ok(record)
}

static START: OnceLock<Instant> = OnceLock::new();
