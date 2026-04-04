use std::fs;

use anyhow::{Context, Result};

use crate::{paths::PraxisPaths, storage::SqliteSessionStore};

use super::OpportunityStatus;

pub(super) fn sync(paths: &PraxisPaths, store: &SqliteSessionStore) -> Result<()> {
    let pending = store.list_opportunities(OpportunityStatus::Pending, 100)?;
    let accepted = store.list_opportunities(OpportunityStatus::Accepted, 100)?;
    let dismissed = store.list_opportunities(OpportunityStatus::Dismissed, 100)?;
    let mut sections = vec!["# Proposals".to_string()];
    sections.extend(render_section(OpportunityStatus::Pending, &pending));
    sections.extend(render_section(OpportunityStatus::Accepted, &accepted));
    sections.extend(render_section(OpportunityStatus::Dismissed, &dismissed));

    fs::write(&paths.proposals_file, sections.join("\n"))
        .with_context(|| format!("failed to write {}", paths.proposals_file.display()))?;
    Ok(())
}

fn render_section(
    status: OpportunityStatus,
    opportunities: &[crate::learning::StoredOpportunity],
) -> Vec<String> {
    let mut lines = vec![String::new(), format!("## {}", status.heading())];
    if opportunities.is_empty() {
        lines.push(format!("- No {} proposals.", status.as_str()));
        return lines;
    }

    lines.extend(opportunities.iter().map(|item| {
        format!(
            "- [#{}] {} ({})\n  status: {}\n  created_at: {}\n  summary: {}",
            item.id, item.title, item.kind, item.status, item.created_at, item.summary
        )
    }));
    lines
}
