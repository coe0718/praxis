use std::{env, str::FromStr};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;

pub trait Clock {
    fn now_utc(&self) -> DateTime<Utc>;
}

#[derive(Debug, Clone)]
pub struct SystemClock {
    fixed_now: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct FixedClock {
    now: DateTime<Utc>,
}

impl SystemClock {
    pub fn from_env() -> Result<Self> {
        let fixed_now = match env::var("PRAXIS_FIXED_NOW") {
            Ok(value) => Some(
                DateTime::parse_from_rfc3339(&value)
                    .with_context(|| format!("invalid PRAXIS_FIXED_NOW value {value}"))?
                    .with_timezone(&Utc),
            ),
            Err(env::VarError::NotPresent) => None,
            Err(error) => return Err(error.into()),
        };

        Ok(Self { fixed_now })
    }
}

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for SystemClock {
    fn now_utc(&self) -> DateTime<Utc> {
        self.fixed_now.as_ref().cloned().unwrap_or_else(Utc::now)
    }
}

impl Clock for FixedClock {
    fn now_utc(&self) -> DateTime<Utc> {
        self.now
    }
}

pub fn parse_timezone(name: &str) -> Result<Tz> {
    Tz::from_str(name).with_context(|| format!("invalid timezone {name}"))
}

pub fn parse_clock_time(value: &str) -> Result<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .with_context(|| format!("invalid quiet-hours time {value}, expected HH:MM"))
}

pub fn is_quiet_hours(
    now_utc: DateTime<Utc>,
    timezone: &str,
    quiet_hours_start: &str,
    quiet_hours_end: &str,
) -> Result<bool> {
    let tz = parse_timezone(timezone)?;
    let local_time = now_utc.with_timezone(&tz).time();
    let start = parse_clock_time(quiet_hours_start)?;
    let end = parse_clock_time(quiet_hours_end)?;

    if start == end {
        return Ok(false);
    }

    if start < end {
        Ok(local_time >= start && local_time < end)
    } else {
        Ok(local_time >= start || local_time < end)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{FixedClock, is_quiet_hours};
    use crate::time::Clock;

    #[test]
    fn detects_quiet_hours_across_midnight() {
        let clock = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 3, 31, 4, 30, 0).unwrap());

        let quiet = is_quiet_hours(clock.now_utc(), "UTC", "23:00", "07:00").unwrap();
        assert!(quiet);
    }

    #[test]
    fn ignores_quiet_hours_outside_window() {
        let clock = FixedClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 3, 31, 12, 30, 0)
                .unwrap(),
        );

        let quiet = is_quiet_hours(clock.now_utc(), "UTC", "23:00", "07:00").unwrap();
        assert!(!quiet);
    }
}
