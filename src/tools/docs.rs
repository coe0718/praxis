use std::{collections::BTreeMap, fs};

use anyhow::{Context, Result};

use crate::{
    paths::PraxisPaths,
    storage::{ApprovalStatus, ApprovalStore, StoredApprovalRequest},
};

use super::{ToolManifest, ToolRegistry};

pub fn sync_capabilities<R: ToolRegistry, S: ApprovalStore>(
    registry: &R,
    store: &S,
    paths: &PraxisPaths,
) -> Result<()> {
    let manifests = registry.list(paths)?;
    let approvals = store.list_approvals(None)?;
    let rendered = render_capabilities(&manifests, &approvals);
    fs::write(&paths.capabilities_file, rendered)
        .with_context(|| format!("failed to write {}", paths.capabilities_file.display()))
}

fn render_capabilities(manifests: &[ToolManifest], approvals: &[StoredApprovalRequest]) -> String {
    let grouped = approvals.iter().fold(
        BTreeMap::<&str, Vec<&StoredApprovalRequest>>::new(),
        |mut acc, item| {
            acc.entry(&item.tool_name).or_default().push(item);
            acc
        },
    );

    let mut lines = vec![
        "# Capabilities".to_string(),
        String::new(),
        "Generated automatically from the current tool registry and request history.".to_string(),
        String::new(),
        "## Tools".to_string(),
    ];

    for manifest in manifests {
        lines.push(String::new());
        lines.push(format!("### {}", manifest.name));
        lines.push(format!("- Description: {}", manifest.description));
        lines.push(format!("- Kind: {}", manifest.kind.as_str()));
        lines.push(format!("- Required level: {}", manifest.required_level));
        lines.push(format!(
            "- Approval: {}",
            yes_no(manifest.requires_approval)
        ));
        lines.push(format!(
            "- Rehearsal: {}",
            yes_no(manifest.rehearsal_required)
        ));
        lines.push(format!(
            "- Allowed paths: {}",
            comma_or_none(&manifest.allowed_paths)
        ));
        lines.push(format!(
            "- Reliability: {}",
            reliability_line(grouped.get(manifest.name.as_str()).map(Vec::as_slice))
        ));
        lines.push("- Recent examples:".to_string());
        lines.extend(example_lines(
            grouped.get(manifest.name.as_str()).map(Vec::as_slice),
        ));
        lines.push("- Failure modes:".to_string());
        lines.extend(failure_lines(
            grouped.get(manifest.name.as_str()).map(Vec::as_slice),
        ));
    }

    lines.join("\n")
}

fn reliability_line(history: Option<&[&StoredApprovalRequest]>) -> String {
    let Some(history) = history else {
        return "no request history yet".to_string();
    };
    let pending = count_status(history, ApprovalStatus::Pending);
    let approved = count_status(history, ApprovalStatus::Approved);
    let executed = count_status(history, ApprovalStatus::Executed);
    let rejected = count_status(history, ApprovalStatus::Rejected);

    format!("executed={executed} approved={approved} pending={pending} rejected={rejected}")
}

fn example_lines(history: Option<&[&StoredApprovalRequest]>) -> Vec<String> {
    let Some(history) = history else {
        return vec!["  - No requests recorded yet.".to_string()];
    };
    let mut items = history.to_vec();
    items.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    items.truncate(3);
    items
        .into_iter()
        .map(|item| {
            format!(
                "  - [{} @ {}] {}",
                item.status.as_str(),
                item.updated_at,
                item.summary
            )
        })
        .collect()
}

fn failure_lines(history: Option<&[&StoredApprovalRequest]>) -> Vec<String> {
    let Some(history) = history else {
        return vec!["  - No rejected requests recorded yet.".to_string()];
    };
    let failures = history
        .iter()
        .filter(|item| item.status == ApprovalStatus::Rejected)
        .take(3)
        .map(|item| {
            item.status_note.as_ref().map_or_else(
                || format!("  - Rejected request: {}", item.summary),
                |note| format!("  - Rejected request: {} ({note})", item.summary),
            )
        })
        .collect::<Vec<_>>();

    if failures.is_empty() {
        vec!["  - No rejected requests recorded yet.".to_string()]
    } else {
        failures
    }
}

fn count_status(history: &[&StoredApprovalRequest], status: ApprovalStatus) -> usize {
    history.iter().filter(|item| item.status == status).count()
}

fn comma_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "required" } else { "not required" }
}
