use super::{LearningRunSummary, OpportunityStatus, StoredLearningRun, StoredOpportunity};

pub fn render_run(summary: &LearningRunSummary) -> String {
    let mut lines = vec![
        "learning: ok".to_string(),
        format!("sources_processed: {}", summary.processed_sources),
        format!("sources_changed: {}", summary.changed_sources),
        format!("opportunities_created: {}", summary.opportunities_created),
        format!("throttle_reached: {}", summary.throttle_reached),
    ];
    if summary.notes.is_empty() {
        lines.push("notes: none".to_string());
    } else {
        lines.push("notes:".to_string());
        lines.extend(summary.notes.iter().map(|note| format!("- {note}")));
    }
    lines.join("\n")
}

pub fn render_list(
    last_run: Option<StoredLearningRun>,
    pending: &[StoredOpportunity],
    accepted: &[StoredOpportunity],
    dismissed: &[StoredOpportunity],
    include_all: bool,
) -> String {
    let mut lines = vec!["learning: ready".to_string()];
    if let Some(run) = last_run {
        lines.push(format!(
            "last_run: processed={} changed={} opportunities={} completed_at={}",
            run.processed_sources, run.changed_sources, run.opportunities_created, run.completed_at
        ));
    } else {
        lines.push("last_run: none".to_string());
    }

    lines.push(format!("pending_opportunities: {}", pending.len()));
    if include_all {
        lines.push(format!("accepted_opportunities: {}", accepted.len()));
        lines.push(format!("dismissed_opportunities: {}", dismissed.len()));
    }
    append_section(&mut lines, OpportunityStatus::Pending, pending);
    if include_all {
        append_section(&mut lines, OpportunityStatus::Accepted, accepted);
        append_section(&mut lines, OpportunityStatus::Dismissed, dismissed);
    }
    lines.join("\n")
}

pub fn render_action(action: &str, opportunity: &StoredOpportunity) -> String {
    [
        "learning: updated".to_string(),
        format!("action: {action}"),
        format!("opportunity_id: {}", opportunity.id),
        format!("status: {}", opportunity.status),
        format!("title: {}", opportunity.title),
        format!("summary: {}", opportunity.summary),
    ]
    .join("\n")
}

fn append_section(
    lines: &mut Vec<String>,
    status: OpportunityStatus,
    opportunities: &[StoredOpportunity],
) {
    let key = status.as_str();
    if opportunities.is_empty() {
        lines.push(format!("{key}: none"));
        return;
    }
    lines.push(format!("{key}:"));
    lines.extend(
        opportunities
            .iter()
            .map(|item| format!("- #{} [{}] {}", item.id, item.kind, item.title)),
    );
}
