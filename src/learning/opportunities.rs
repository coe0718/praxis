use anyhow::Result;
use chrono::{DateTime, Duration, Utc};

use crate::{
    argus::{ArgusReport, DriftStatus},
    learning::OpportunityCandidate,
    paths::PraxisPaths,
    storage::SqliteSessionStore,
};

const DAILY_LIMIT: usize = 2;
const WEEKLY_LIMIT: usize = 5;

pub(super) struct OpportunityMiningResult {
    pub created: usize,
    pub throttle_reached: bool,
    pub notes: Vec<String>,
}

pub(super) fn mine(
    _paths: &PraxisPaths,
    store: &SqliteSessionStore,
    report: &ArgusReport,
    now: DateTime<Utc>,
) -> Result<OpportunityMiningResult> {
    let mut slots = remaining_slots(store, now)?;
    let mut created = 0;
    let mut notes = Vec::new();
    let mut throttle_reached = false;

    for candidate in candidates(report)? {
        if store.has_opportunity_signature(&candidate.signature)? {
            continue;
        }
        if slots == 0 {
            throttle_reached = true;
            break;
        }
        store.create_opportunity(&candidate, now)?;
        created += 1;
        slots -= 1;
        notes.push(format!("Queued opportunity: {}.", candidate.title));
    }

    Ok(OpportunityMiningResult {
        created,
        throttle_reached,
        notes,
    })
}

fn candidates(report: &ArgusReport) -> Result<Vec<OpportunityCandidate>> {
    let mut items = Vec::new();
    if report.drift.status == DriftStatus::Regressed {
        items.push(OpportunityCandidate {
            signature: "drift:regressed".to_string(),
            kind: "stability".to_string(),
            title: "Stabilize runtime quality drift".to_string(),
            summary: format!(
                "Recent session quality regressed from {:.2} to {:.2}. Review failures, eval failures, or loop guard blocks are climbing.",
                report.drift.baseline_score, report.drift.recent_score
            ),
            evidence_json: serde_json::json!({
                "recent_score": report.drift.recent_score,
                "baseline_score": report.drift.baseline_score,
                "status": report.drift.status.as_str()
            })
            .to_string(),
        });
    }

    for pattern in report.repeated_work.iter().filter(|pattern| pattern.distinct_days >= 2) {
        items.push(OpportunityCandidate {
            signature: format!("repeated-work:{}", pattern.label.to_lowercase()),
            kind: "automation".to_string(),
            title: format!("Automate recurring work: {}", pattern.label),
            summary: format!(
                "{} resurfaced across {} sessions on {} days. Latest outcome: {}.",
                pattern.label, pattern.sessions, pattern.distinct_days, pattern.latest_outcome
            ),
            evidence_json: serde_json::json!({
                "label": pattern.label,
                "sessions": pattern.sessions,
                "days": pattern.distinct_days,
                "latest_outcome": pattern.latest_outcome
            })
            .to_string(),
        });
    }

    Ok(items)
}

fn remaining_slots(store: &SqliteSessionStore, now: DateTime<Utc>) -> Result<usize> {
    let daily_count = store.count_opportunities_since(now - Duration::days(1))? as usize;
    let weekly_count = store.count_opportunities_since(now - Duration::days(7))? as usize;
    let daily_remaining = DAILY_LIMIT.saturating_sub(daily_count);
    let weekly_remaining = WEEKLY_LIMIT.saturating_sub(weekly_count);
    Ok(daily_remaining.min(weekly_remaining))
}
