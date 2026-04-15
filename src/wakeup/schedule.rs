//! Adaptive scheduling — learns operator activity patterns from interaction
//! timestamps and uses them to suggest wake times that avoid quiet hours.
//!
//! The schedule is a histogram of hourly interaction counts (hours 0-23 UTC).
//! After enough samples, hours with fewer than `QUIET_THRESHOLD_PCT` of the
//! peak hour's activity are treated as quiet hours and skipped when choosing
//! the next preferred wake time.
//!
//! The data is persisted in `operator_schedule.json`.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Minimum samples before quiet-hour detection activates.
const MIN_SAMPLES: u64 = 20;

/// Hours below this fraction of peak activity are treated as quiet.
const QUIET_THRESHOLD_PCT: f64 = 0.10;

/// Operator interaction histogram by UTC hour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorSchedule {
    /// Interaction counts per UTC hour (indices 0-23).
    #[serde(default)]
    pub hourly_counts: [u64; 24],
    /// Total recorded interactions.
    #[serde(default)]
    pub total_samples: u64,
}

impl Default for OperatorSchedule {
    fn default() -> Self {
        Self {
            hourly_counts: [0; 24],
            total_samples: 0,
        }
    }
}

impl OperatorSchedule {
    pub fn load(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(raw) => serde_json::from_str(&raw)
                .with_context(|| format!("invalid schedule at {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw =
            serde_json::to_string_pretty(self).context("failed to serialize operator schedule")?;
        fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
    }

    /// Record an operator interaction at the given UTC timestamp.
    pub fn record_activity(&mut self, at: DateTime<Utc>) {
        let hour = at.hour() as usize;
        self.hourly_counts[hour] = self.hourly_counts[hour].saturating_add(1);
        self.total_samples = self.total_samples.saturating_add(1);
    }

    /// Return hours (0-23 UTC) where activity is below the quiet threshold.
    ///
    /// Returns an empty vec if there are not enough samples yet.
    pub fn quiet_hours(&self) -> Vec<u32> {
        if self.total_samples < MIN_SAMPLES {
            return Vec::new();
        }
        let peak = *self.hourly_counts.iter().max().unwrap_or(&1).max(&1);
        let threshold = (peak as f64 * QUIET_THRESHOLD_PCT) as u64;
        (0u32..24)
            .filter(|&h| self.hourly_counts[h as usize] <= threshold)
            .collect()
    }

    /// Find the next non-quiet UTC hour at or after `from`.
    ///
    /// Searches up to 24 hours ahead; falls back to `from + 1 hour` if all
    /// hours are quiet (shouldn't happen in practice).
    pub fn next_preferred_wake_time(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        let quiet = self.quiet_hours();
        for offset_hours in 0..24_i64 {
            let candidate = from + Duration::hours(offset_hours);
            let h = candidate.hour();
            if !quiet.contains(&h) {
                // Round to the start of the hour.
                return candidate
                    .with_minute(0)
                    .and_then(|t| t.with_second(0))
                    .and_then(|t| t.with_nanosecond(0))
                    .unwrap_or(candidate);
            }
        }
        from + Duration::hours(1)
    }

    /// Human-readable summary of the current schedule.
    pub fn summary(&self) -> String {
        let quiet = self.quiet_hours();
        if self.total_samples < MIN_SAMPLES {
            return format!(
                "adaptive schedule: learning ({}/{} samples needed)",
                self.total_samples, MIN_SAMPLES
            );
        }
        if quiet.is_empty() {
            return format!(
                "adaptive schedule: {} samples, no quiet hours detected",
                self.total_samples
            );
        }
        let quiet_str: Vec<String> = quiet.iter().map(|h| format!("{h:02}:00")).collect();
        format!(
            "adaptive schedule: {} samples, quiet hours (UTC): {}",
            self.total_samples,
            quiet_str.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::OperatorSchedule;

    #[test]
    fn no_quiet_hours_without_enough_samples() {
        let mut schedule = OperatorSchedule::default();
        let t = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 10, 0, 0).unwrap();
        for _ in 0..5 {
            schedule.record_activity(t);
        }
        assert!(schedule.quiet_hours().is_empty());
    }

    #[test]
    fn detects_quiet_hours_after_enough_samples() {
        let mut schedule = OperatorSchedule::default();
        // Flood daytime hours 9-17 with activity.
        let base = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 0, 0, 0).unwrap();
        for h in 9u32..=17 {
            let t = base + chrono::Duration::hours(h as i64);
            for _ in 0..5 {
                schedule.record_activity(t);
            }
        }
        let quiet = schedule.quiet_hours();
        // Midnight should be quiet.
        assert!(quiet.contains(&0));
        // Noon should not be quiet.
        assert!(!quiet.contains(&12));
    }
}
