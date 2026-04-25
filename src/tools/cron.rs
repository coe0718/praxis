//! Agent-callable scheduled jobs — the cron tool.
//!
//! When `features.cron_tool = true` in `praxis.toml`, the agent can schedule
//! recurring or one-shot tasks that trigger sessions.  Jobs are persisted in
//! `scheduled_jobs.json` in the data directory.
//!
//! The daemon checks for due jobs every poll cycle and fires a `WakeIntent`
//! with the job's task description when one is ready.
//!
//! Supported schedule formats (deliberately simple — no full cron parser):
//! - `"every Ns"` / `"every Nm"` / `"every Nh"` / `"every Nd"` — interval
//! - `"in Ns"` / `"in Nm"` / `"in Nh"` — one-shot, fires once then auto-removes
//! - `"hourly"` / `"daily"` / `"weekly"` — fixed presets

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Persistent job store backed by a JSON file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduledJobs {
    pub jobs: Vec<ScheduledJob>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    /// Unique identifier (UUID-like string generated at creation).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// The schedule expression (e.g., "every 30m", "in 2h", "daily").
    pub schedule: String,
    /// Parsed schedule kind, stored so we don't re-parse every cycle.
    #[serde(flatten)]
    pub kind: ScheduleKind,
    /// Task description injected into the triggered session.
    pub task: String,
    /// When this job was created.
    pub created_at: DateTime<Utc>,
    /// When this job last fired (None = never fired).
    pub last_fired_at: Option<DateTime<Utc>>,
    /// Next fire time (computed from schedule + last_fired_at).
    pub next_fire_at: DateTime<Utc>,
    /// Whether this job repeats or auto-removes after one fire.
    pub recurring: bool,
    /// How many times this job has fired.
    pub fire_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "schedule_type")]
pub enum ScheduleKind {
    /// Fixed interval in seconds.
    #[serde(rename = "interval")]
    Interval { secs: u64 },
    /// Fixed presets — aligned to hour/day/week boundaries.
    #[serde(rename = "preset")]
    Preset { preset: SchedulePreset },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SchedulePreset {
    Hourly,
    Daily,
    Weekly,
}

impl ScheduledJobs {
    /// Load from the JSON file, returning an empty store if the file is absent.
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
    }

    /// Persist to disk.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize scheduled jobs")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Return all jobs whose `next_fire_at` is in the past, updating their
    /// fire timestamps and removing completed one-shot jobs.
    pub fn drain_due(&mut self, now: DateTime<Utc>) -> Vec<ScheduledJob> {
        let mut due = Vec::new();
        let mut keep = Vec::new();

        for mut job in self.jobs.drain(..) {
            if job.next_fire_at <= now {
                job.last_fired_at = Some(now);
                job.fire_count += 1;
                due.push(job.clone());

                if job.recurring {
                    job.next_fire_at = compute_next_fire(&job.kind, now);
                    keep.push(job);
                }
                // One-shot jobs are NOT kept — auto-removed.
            } else {
                keep.push(job);
            }
        }

        self.jobs = keep;
        due
    }

    /// Add a new job.
    pub fn add(&mut self, job: ScheduledJob) {
        self.jobs.push(job);
    }

    /// Remove a job by ID.
    pub fn remove(&mut self, id: &str) -> bool {
        let before = self.jobs.len();
        self.jobs.retain(|j| j.id != id);
        self.jobs.len() < before
    }

    /// Find a job by ID.
    pub fn get(&self, id: &str) -> Option<&ScheduledJob> {
        self.jobs.iter().find(|j| j.id == id)
    }
}

/// Parse a schedule expression into a `ScheduleKind` and recurrence flag.
pub fn parse_schedule(expr: &str) -> Result<(ScheduleKind, bool)> {
    let lower = expr.to_lowercase();

    // Interval: "every 30m", "every 2h", "every 1d"
    if let Some(rest) = lower.strip_prefix("every ") {
        let secs = parse_duration_suffix(rest.trim())?;
        return Ok((ScheduleKind::Interval { secs }, true));
    }

    // One-shot: "in 30m", "in 2h"
    if let Some(rest) = lower.strip_prefix("in ") {
        let secs = parse_duration_suffix(rest.trim())?;
        return Ok((ScheduleKind::Interval { secs }, false));
    }

    // Presets.
    match lower.as_str() {
        "hourly" => Ok((ScheduleKind::Preset { preset: SchedulePreset::Hourly }, true)),
        "daily" => Ok((ScheduleKind::Preset { preset: SchedulePreset::Daily }, true)),
        "weekly" => Ok((ScheduleKind::Preset { preset: SchedulePreset::Weekly }, true)),
        _ => bail!(
            "unsupported schedule expression: '{expr}'. \
             Use 'every Ns/m/h/d', 'in Ns/m/h', or 'hourly'/'daily'/'weekly'."
        ),
    }
}

/// Parse a duration string like "30m", "2h", "1d", "90s".
fn parse_duration_suffix(s: &str) -> Result<u64> {
    let (num_part, suffix) = if let Some(n) = s.strip_suffix('s') {
        (n, 's')
    } else if let Some(n) = s.strip_suffix('m') {
        (n, 'm')
    } else if let Some(n) = s.strip_suffix('h') {
        (n, 'h')
    } else if let Some(n) = s.strip_suffix('d') {
        (n, 'd')
    } else {
        bail!("schedule duration '{s}' must end with s/m/h/d (e.g. 30m, 2h, 1d)");
    };

    let num: u64 = num_part.parse().with_context(|| format!("invalid number in '{s}'"))?;
    Ok(match suffix {
        's' => num,
        'm' => num * 60,
        'h' => num * 3600,
        'd' => num * 86400,
        _ => unreachable!(),
    })
}

/// Compute the next fire time given a schedule kind and the current time.
fn compute_next_fire(kind: &ScheduleKind, now: DateTime<Utc>) -> DateTime<Utc> {
    match kind {
        ScheduleKind::Interval { secs } => now + chrono::Duration::seconds(*secs as i64),
        ScheduleKind::Preset { preset } => match preset {
            SchedulePreset::Hourly => {
                let next = now + chrono::Duration::hours(1);
                next.with_minute(0).unwrap_or(next).with_second(0).unwrap_or(next)
            }
            SchedulePreset::Daily => {
                let next = now + chrono::Duration::days(1);
                next.with_hour(9)
                    .unwrap_or(next)
                    .with_minute(0)
                    .unwrap_or(next)
                    .with_second(0)
                    .unwrap_or(next)
            }
            SchedulePreset::Weekly => {
                let next = now + chrono::Duration::weeks(1);
                next.with_hour(9)
                    .unwrap_or(next)
                    .with_minute(0)
                    .unwrap_or(next)
                    .with_second(0)
                    .unwrap_or(next)
            }
        },
    }
}

/// Create a new scheduled job with auto-generated ID and computed next fire time.
pub fn create_job(name: String, schedule: String, task: String) -> Result<ScheduledJob> {
    let (kind, recurring) = parse_schedule(&schedule)?;
    let now = Utc::now();
    let next_fire_at = compute_next_fire(&kind, now);
    let id = format!("job-{}", &uuid_or_timestamp());

    Ok(ScheduledJob {
        id,
        name,
        schedule,
        kind,
        task,
        created_at: now,
        last_fired_at: None,
        next_fire_at,
        recurring,
        fire_count: 0,
    })
}

/// Generate a simple unique-enough ID without pulling in the uuid crate.
fn uuid_or_timestamp() -> String {
    use std::time::SystemTime;
    let dur = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    format!("{:x}", dur.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_schedule() {
        let (kind, recurring) = parse_schedule("every 30m").unwrap();
        assert_eq!(kind, ScheduleKind::Interval { secs: 1800 });
        assert!(recurring);
    }

    #[test]
    fn parse_one_shot_schedule() {
        let (kind, recurring) = parse_schedule("in 2h").unwrap();
        assert_eq!(kind, ScheduleKind::Interval { secs: 7200 });
        assert!(!recurring);
    }

    #[test]
    fn parse_preset_daily() {
        let (kind, recurring) = parse_schedule("daily").unwrap();
        assert_eq!(kind, ScheduleKind::Preset { preset: SchedulePreset::Daily });
        assert!(recurring);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_schedule("tomorrow at 3pm").is_err());
        assert!(parse_schedule("").is_err());
    }

    #[test]
    fn drain_due_fires_and_updates() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let mut jobs = ScheduledJobs {
            jobs: vec![ScheduledJob {
                id: "job-test".to_string(),
                name: "test".to_string(),
                schedule: "every 30m".to_string(),
                kind: ScheduleKind::Interval { secs: 1800 },
                task: "do something".to_string(),
                created_at: past,
                last_fired_at: None,
                next_fire_at: past, // Due now.
                recurring: true,
                fire_count: 0,
            }],
        };

        let due = jobs.drain_due(now);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].fire_count, 1);
        // Recurring job should be kept with updated next_fire_at.
        assert_eq!(jobs.jobs.len(), 1);
        assert!(jobs.jobs[0].next_fire_at > now);
    }

    #[test]
    fn drain_due_removes_one_shot() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(1);
        let mut jobs = ScheduledJobs {
            jobs: vec![ScheduledJob {
                id: "job-oneshot".to_string(),
                name: "once".to_string(),
                schedule: "in 1h".to_string(),
                kind: ScheduleKind::Interval { secs: 3600 },
                task: "do once".to_string(),
                created_at: past,
                last_fired_at: None,
                next_fire_at: past,
                recurring: false,
                fire_count: 0,
            }],
        };

        let due = jobs.drain_due(now);
        assert_eq!(due.len(), 1);
        // One-shot job should be auto-removed.
        assert!(jobs.jobs.is_empty());
    }

    #[test]
    fn create_job_roundtrip() {
        let job =
            create_job("test job".to_string(), "every 1h".to_string(), "check email".to_string())
                .unwrap();
        assert!(job.id.starts_with("job-"));
        assert!(job.recurring);
        assert_eq!(job.fire_count, 0);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("scheduled_jobs.json");

        let mut store = ScheduledJobs::default();
        let job =
            create_job("daily standup".into(), "daily".into(), "review goals".into()).unwrap();
        store.add(job);
        store.save(&path).unwrap();

        let loaded = ScheduledJobs::load(&path).unwrap();
        assert_eq!(loaded.jobs.len(), 1);
        assert_eq!(loaded.jobs[0].name, "daily standup");
    }
}
