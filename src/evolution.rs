//! Meta-evolution workflow — Praxis proposes changes to its own configuration,
//! profiles, or identity through a heavily gated approval process.
//!
//! ## Workflow
//!
//! 1. The agent writes a proposal to the `EvolutionStore` (typically from
//!    Reflect after spotting a recurring pattern or opportunity).
//! 2. The operator reviews proposals with `praxis evolve list/show`.
//! 3. The operator approves with `praxis evolve approve <id>`.
//! 4. The operator **applies** with `praxis evolve apply <id>`.  Only
//!    `Config` and `Profile` proposals can be applied automatically.
//!    `Identity` and `Code` proposals emit human-readable instructions.
//! 5. Applied proposals are marked `Applied`; rejected ones are `Rejected`.
//!
//! ## Safety guarantees
//!
//! - No proposal is applied without two explicit operator commands
//!   (`approve` then `apply`).
//! - `Code` proposals are **never** applied automatically — they emit a diff
//!   or description the operator must act on manually.
//! - The store uses an append-only JSONL log (`evolution.jsonl`) so the full
//!   proposal history is always auditable.
//! - `SELF_EVOLUTION.md` is regenerated from the store and is read-only from
//!   the agent's perspective.

use std::{
    fs::{self, OpenOptions},
    io::Write as IoWrite,
    path::Path,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Types ─────────────────────────────────────────────────────────────────────

/// The kind of change being proposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Modify a value in `praxis.toml` (`[agent]`, `[runtime]`, etc.).
    Config,
    /// Add or modify a named profile in `profiles.toml`.
    Profile,
    /// Update `IDENTITY.md`, `GOALS.md`, `SOUL.md`, or `PATTERNS.md`.
    Identity,
    /// Change to the Praxis source code — human must apply manually.
    Code,
}

impl ChangeKind {
    pub fn is_auto_applicable(self) -> bool {
        matches!(self, Self::Config | Self::Profile)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Profile => "profile",
            Self::Identity => "identity",
            Self::Code => "code (manual)",
        }
    }
}

/// Lifecycle state of a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    /// Written by the agent, awaiting operator review.
    Proposed,
    /// Operator has approved; not yet applied.
    Approved,
    /// Operator has applied the change.
    Applied,
    /// Operator rejected the proposal.
    Rejected,
}

impl ProposalStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Applied => "applied",
            Self::Rejected => "rejected",
        }
    }
}

/// A single evolution proposal written by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionProposal {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub status: ProposalStatus,
    /// One-line summary.
    pub title: String,
    /// Why the agent thinks this change is beneficial.
    pub motivation: String,
    /// What kind of change is being proposed.
    pub change_kind: ChangeKind,
    /// The actual change payload — varies by `change_kind`:
    /// - Config: TOML snippet to merge
    /// - Profile: full profile TOML block
    /// - Identity: markdown patch or new section text
    /// - Code: human-readable description / diff context
    pub change_details: String,
    /// Session IDs whose outcomes support this proposal.
    #[serde(default)]
    pub evidence_session_ids: Vec<i64>,
    #[serde(default)]
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub applied_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub rejection_note: Option<String>,
}

impl EvolutionProposal {
    /// Generate a new proposal with a stable ID derived from the timestamp.
    pub fn new(
        title: impl Into<String>,
        motivation: impl Into<String>,
        change_kind: ChangeKind,
        change_details: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        let id = format!("evo-{}", now.format("%Y%m%d-%H%M%S"));
        Self {
            id,
            created_at: now,
            status: ProposalStatus::Proposed,
            title: title.into(),
            motivation: motivation.into(),
            change_kind,
            change_details: change_details.into(),
            evidence_session_ids: Vec::new(),
            approved_at: None,
            applied_at: None,
            rejection_note: None,
        }
    }

    pub fn with_evidence(mut self, session_ids: Vec<i64>) -> Self {
        self.evidence_session_ids = session_ids;
        self
    }

    /// One-line display summary.
    pub fn summary_line(&self) -> String {
        format!(
            "[{}] {} ({}) — {}",
            self.status.label(),
            self.id,
            self.change_kind.label(),
            self.title,
        )
    }
}

// ── Store ─────────────────────────────────────────────────────────────────────

/// Append-only JSONL store for evolution proposals.
pub struct EvolutionStore {
    path: std::path::PathBuf,
}

impl EvolutionStore {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    pub fn from_paths(paths: &PraxisPaths) -> Self {
        Self::new(paths.evolution_file.clone())
    }

    /// Append a new proposal to the log.
    pub fn propose(&self, proposal: &EvolutionProposal) -> Result<()> {
        self.append(proposal)
    }

    /// Load all proposals in chronological order.
    pub fn all(&self) -> Result<Vec<EvolutionProposal>> {
        load_all(&self.path)
    }

    /// Load only proposals matching a status filter.
    pub fn with_status(&self, status: ProposalStatus) -> Result<Vec<EvolutionProposal>> {
        Ok(self.all()?.into_iter().filter(|p| p.status == status).collect())
    }

    /// Find a proposal by ID.
    pub fn get(&self, id: &str) -> Result<Option<EvolutionProposal>> {
        Ok(self.all()?.into_iter().find(|p| p.id == id))
    }

    /// Approve a proposal.  Only `Proposed` proposals can be approved.
    pub fn approve(&self, id: &str) -> Result<EvolutionProposal> {
        let mut proposals = self.all()?;
        let proposal = proposals
            .iter_mut()
            .find(|p| p.id == id)
            .with_context(|| format!("evolution proposal '{id}' not found"))?;
        if proposal.status != ProposalStatus::Proposed {
            bail!(
                "proposal '{id}' is '{}' — only Proposed proposals can be approved",
                proposal.status.label()
            );
        }
        proposal.status = ProposalStatus::Approved;
        proposal.approved_at = Some(Utc::now());
        let updated = proposal.clone();
        rewrite_all(&self.path, &proposals)?;
        Ok(updated)
    }

    /// Reject a proposal.
    pub fn reject(&self, id: &str, note: impl Into<String>) -> Result<EvolutionProposal> {
        let mut proposals = self.all()?;
        let proposal = proposals
            .iter_mut()
            .find(|p| p.id == id)
            .with_context(|| format!("evolution proposal '{id}' not found"))?;
        if proposal.status == ProposalStatus::Applied {
            bail!("proposal '{id}' has already been applied — cannot reject");
        }
        proposal.status = ProposalStatus::Rejected;
        proposal.rejection_note = Some(note.into());
        let updated = proposal.clone();
        rewrite_all(&self.path, &proposals)?;
        Ok(updated)
    }

    /// Mark a proposal as applied (called after the change is made).
    pub fn mark_applied(&self, id: &str) -> Result<EvolutionProposal> {
        let mut proposals = self.all()?;
        let proposal = proposals
            .iter_mut()
            .find(|p| p.id == id)
            .with_context(|| format!("evolution proposal '{id}' not found"))?;
        if proposal.status != ProposalStatus::Approved {
            bail!(
                "proposal '{id}' is '{}' — cannot be applied (must be Approved first)",
                proposal.status.label()
            );
        }
        proposal.status = ProposalStatus::Applied;
        proposal.applied_at = Some(Utc::now());
        let updated = proposal.clone();
        rewrite_all(&self.path, &proposals)?;
        Ok(updated)
    }

    fn append(&self, proposal: &EvolutionProposal) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let line = serde_json::to_string(proposal).context("failed to serialise proposal")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("failed to open {}", self.path.display()))?;
        writeln!(file, "{line}").with_context(|| format!("failed to write {}", self.path.display()))
    }
}

fn load_all(path: &Path) -> Result<Vec<EvolutionProposal>> {
    match fs::read_to_string(path) {
        Ok(raw) => {
            let proposals = raw
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            Ok(proposals)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn rewrite_all(path: &Path, proposals: &[EvolutionProposal]) -> Result<()> {
    let mut content = String::new();
    for p in proposals {
        content.push_str(&serde_json::to_string(p).context("failed to serialise proposal")?);
        content.push('\n');
    }
    fs::write(path, content).with_context(|| format!("failed to rewrite {}", path.display()))
}

// ── SELF_EVOLUTION.md renderer ────────────────────────────────────────────────

/// Regenerate `SELF_EVOLUTION.md` from the current store contents.
pub fn render_self_evolution_doc(paths: &PraxisPaths) -> Result<()> {
    let store = EvolutionStore::from_paths(paths);
    let proposals = store.all()?;

    let mut doc = String::from(
        "# Self-Evolution Proposals\n\n\
         > Generated from `evolution.jsonl` — do not edit manually.\n\n",
    );

    let pending: Vec<_> = proposals
        .iter()
        .filter(|p| matches!(p.status, ProposalStatus::Proposed | ProposalStatus::Approved))
        .collect();
    let historical: Vec<_> = proposals
        .iter()
        .filter(|p| matches!(p.status, ProposalStatus::Applied | ProposalStatus::Rejected))
        .collect();

    if pending.is_empty() {
        doc.push_str("## Pending Proposals\n\n*No pending proposals.*\n\n");
    } else {
        doc.push_str(&format!("## Pending Proposals ({})\n\n", pending.len()));
        for p in &pending {
            doc.push_str(&render_proposal(p));
        }
    }

    if !historical.is_empty() {
        doc.push_str(&format!("\n## History ({})\n\n", historical.len()));
        for p in &historical {
            doc.push_str(&render_proposal_summary(p));
        }
    }

    fs::write(&paths.self_evolution_file, doc)
        .with_context(|| format!("failed to write {}", paths.self_evolution_file.display()))
}

fn render_proposal(p: &EvolutionProposal) -> String {
    let evidence = if p.evidence_session_ids.is_empty() {
        "none".to_string()
    } else {
        p.evidence_session_ids
            .iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "### {} `[{}]`\n\n\
         - **Kind:** {}\n\
         - **Status:** {}\n\
         - **Created:** {}\n\
         - **Evidence sessions:** {evidence}\n\n\
         **Motivation:** {}\n\n\
         **Change:**\n```\n{}\n```\n\n",
        p.title,
        p.id,
        p.change_kind.label(),
        p.status.label(),
        p.created_at.format("%Y-%m-%d %H:%M UTC"),
        p.motivation,
        p.change_details,
    )
}

fn render_proposal_summary(p: &EvolutionProposal) -> String {
    let extra = match p.status {
        ProposalStatus::Applied => p
            .applied_at
            .map(|t| format!(" (applied {})", t.format("%Y-%m-%d")))
            .unwrap_or_default(),
        ProposalStatus::Rejected => {
            p.rejection_note.as_deref().map(|n| format!(" — {n}")).unwrap_or_default()
        }
        _ => String::new(),
    };
    format!("- `{}` **{}** [{}]{extra}\n", p.id, p.title, p.status.label())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn propose_approve_apply_lifecycle() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("evolution.jsonl");
        let store = EvolutionStore::new(path.clone());

        let proposal = EvolutionProposal::new(
            "Increase context window",
            "Sessions regularly hit 90% pressure",
            ChangeKind::Config,
            "[context]\nwindow_tokens = 16000",
        );
        let id = proposal.id.clone();

        store.propose(&proposal).unwrap();
        assert_eq!(store.all().unwrap().len(), 1);

        let approved = store.approve(&id).unwrap();
        assert_eq!(approved.status, ProposalStatus::Approved);

        let applied = store.mark_applied(&id).unwrap();
        assert_eq!(applied.status, ProposalStatus::Applied);
        assert!(applied.applied_at.is_some());
    }

    #[test]
    fn reject_prevents_apply() {
        let dir = TempDir::new().unwrap();
        let store = EvolutionStore::new(dir.path().join("ev.jsonl"));

        let p = EvolutionProposal::new("test", "reason", ChangeKind::Code, "do something");
        let id = p.id.clone();
        store.propose(&p).unwrap();
        store.approve(&id).unwrap();
        store.reject(&id, "not ready").unwrap();

        let err = store.mark_applied(&id).unwrap_err();
        assert!(err.to_string().contains("cannot be applied"));
    }
}
