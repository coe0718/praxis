use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::{
    evolution::{
        ChangeKind, EvolutionProposal, EvolutionStore, ProposalStatus, render_self_evolution_doc,
    },
    paths::{PraxisPaths, default_data_dir},
};

#[derive(Debug, Args)]
pub struct EvolveArgs {
    #[command(subcommand)]
    command: EvolveCommands,
}

#[derive(Debug, Subcommand)]
enum EvolveCommands {
    /// List all proposals (or filter by status).
    List(ListArgs),
    /// Show full details for one proposal.
    Show(ProposalIdArgs),
    /// Write a new proposal (operator-authored).
    Propose(ProposeArgs),
    /// Approve a proposal (first gating step).
    Approve(ProposalIdArgs),
    /// Reject a proposal.
    Reject(RejectArgs),
    /// Apply an approved proposal.
    Apply(ProposalIdArgs),
    /// Regenerate SELF_EVOLUTION.md from the current store.
    Render,
}

#[derive(Debug, Args)]
struct ListArgs {
    /// Filter by status: proposed, approved, applied, rejected.
    #[arg(long)]
    status: Option<String>,
}

#[derive(Debug, Args)]
struct ProposalIdArgs {
    /// Proposal ID (e.g. `evo-20260415-120000`).
    id: String,
}

#[derive(Debug, Args)]
struct ProposeArgs {
    /// One-line title for the proposal.
    #[arg(long)]
    title: String,
    /// Why this change is beneficial.
    #[arg(long)]
    motivation: String,
    /// Change kind: config, profile, identity, or code.
    #[arg(long, default_value = "config")]
    kind: String,
    /// The change details (TOML snippet, markdown patch, or description).
    #[arg(long)]
    details: String,
}

#[derive(Debug, Args)]
struct RejectArgs {
    /// Proposal ID.
    id: String,
    /// Reason for rejection.
    #[arg(long, default_value = "operator rejected")]
    note: String,
}

pub(super) fn handle_evolve(
    data_dir_override: Option<PathBuf>,
    args: EvolveArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = EvolutionStore::from_paths(&paths);

    match args.command {
        EvolveCommands::List(a) => {
            let proposals = if let Some(status_str) = a.status {
                let status = parse_status(&status_str)?;
                store.with_status(status)?
            } else {
                store.all()?
            };
            if proposals.is_empty() {
                return Ok("evolve: no proposals".to_string());
            }
            Ok(proposals.iter().map(|p| p.summary_line()).collect::<Vec<_>>().join("\n"))
        }

        EvolveCommands::Show(a) => match store.get(&a.id)? {
            None => Ok(format!("evolve: proposal '{}' not found", a.id)),
            Some(p) => {
                let evidence = if p.evidence_session_ids.is_empty() {
                    "none".to_string()
                } else {
                    p.evidence_session_ids
                        .iter()
                        .map(|id| format!("#{id}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                Ok(format!(
                    "id: {}\nstatus: {}\nkind: {}\ncreated: {}\ntitle: {}\nevidence: {evidence}\n\nmotivation:\n{}\n\nchange:\n{}",
                    p.id,
                    p.status.label(),
                    p.change_kind.label(),
                    p.created_at.format("%Y-%m-%d %H:%M UTC"),
                    p.title,
                    p.motivation,
                    p.change_details,
                ))
            }
        },

        EvolveCommands::Propose(a) => {
            let kind = parse_kind(&a.kind)?;
            let proposal = EvolutionProposal::new(&a.title, &a.motivation, kind, &a.details);
            let id = proposal.id.clone();
            store.propose(&proposal)?;
            render_self_evolution_doc(&paths)?;
            Ok(format!("evolve: proposal '{id}' written\nreview with: praxis evolve show {id}"))
        }

        EvolveCommands::Approve(a) => {
            let p = store.approve(&a.id)?;
            render_self_evolution_doc(&paths)?;
            Ok(format!(
                "evolve: '{}' approved\napply with: praxis evolve apply {}",
                p.title, p.id
            ))
        }

        EvolveCommands::Reject(a) => {
            store.reject(&a.id, &a.note)?;
            render_self_evolution_doc(&paths)?;
            Ok(format!("evolve: proposal '{}' rejected", a.id))
        }

        EvolveCommands::Apply(a) => {
            let p = store
                .get(&a.id)?
                .ok_or_else(|| anyhow::anyhow!("proposal '{}' not found", a.id))?;

            if p.status != ProposalStatus::Approved {
                anyhow::bail!(
                    "proposal '{}' is '{}' — approve it first with: praxis evolve approve {}",
                    a.id,
                    p.status.label(),
                    a.id
                );
            }

            let instructions = apply_proposal(&paths, &p)?;
            store.mark_applied(&a.id)?;
            render_self_evolution_doc(&paths)?;
            Ok(instructions)
        }

        EvolveCommands::Render => {
            render_self_evolution_doc(&paths)?;
            Ok(format!(
                "evolve: SELF_EVOLUTION.md regenerated → {}",
                paths.self_evolution_file.display()
            ))
        }
    }
}

/// Execute the change described by `proposal`.  Returns a human-readable
/// outcome string.
fn apply_proposal(paths: &PraxisPaths, proposal: &EvolutionProposal) -> Result<String> {
    use crate::evolution::ChangeKind;
    match proposal.change_kind {
        ChangeKind::Config => apply_config_proposal(paths, proposal),
        ChangeKind::Profile => apply_profile_proposal(paths, proposal),
        ChangeKind::Identity => Ok(format!(
            "evolve: identity proposal '{}' cannot be auto-applied.\n\
             Apply manually to the appropriate identity file, then re-run:\n\n{}\n",
            proposal.id, proposal.change_details
        )),
        ChangeKind::Code => Ok(format!(
            "evolve: code proposal '{}' requires manual application.\n\
             Description:\n\n{}\n",
            proposal.id, proposal.change_details
        )),
    }
}

fn apply_config_proposal(paths: &PraxisPaths, proposal: &EvolutionProposal) -> Result<String> {
    // Parse the change_details as a TOML partial config and merge it.
    // For safety, we only apply a whitelist of safe keys via a helper.
    // Full config merge would require a diff/patch library; for now we
    // append the snippet as a commented change note and instruct the operator.
    let note = format!(
        "\n# Evolution proposal {} ({})\n# {}\n# Apply this change to praxis.toml:\n{}\n",
        proposal.id,
        proposal.created_at.format("%Y-%m-%d"),
        proposal.title,
        proposal.change_details,
    );
    // Write the note to a sidecar file rather than mutating praxis.toml directly.
    let pending_path = paths.data_dir.join("evolution_pending.toml");
    std::fs::write(&pending_path, note)?;
    Ok(format!(
        "evolve: config proposal '{}' written to {}\n\
         Review and merge it into praxis.toml manually.",
        proposal.id,
        pending_path.display()
    ))
}

fn apply_profile_proposal(paths: &PraxisPaths, proposal: &EvolutionProposal) -> Result<String> {
    // Similarly, write a sidecar for operator review.
    let pending_path = paths.data_dir.join("evolution_profile_pending.toml");
    std::fs::write(&pending_path, &proposal.change_details)?;
    Ok(format!(
        "evolve: profile proposal '{}' written to {}\n\
         Review and merge it into profiles.toml manually.",
        proposal.id,
        pending_path.display()
    ))
}

fn parse_status(s: &str) -> Result<ProposalStatus> {
    match s {
        "proposed" => Ok(ProposalStatus::Proposed),
        "approved" => Ok(ProposalStatus::Approved),
        "applied" => Ok(ProposalStatus::Applied),
        "rejected" => Ok(ProposalStatus::Rejected),
        other => {
            anyhow::bail!("unknown status '{other}'; use proposed, approved, applied, or rejected")
        }
    }
}

fn parse_kind(s: &str) -> Result<ChangeKind> {
    match s {
        "config" => Ok(ChangeKind::Config),
        "profile" => Ok(ChangeKind::Profile),
        "identity" => Ok(ChangeKind::Identity),
        "code" => Ok(ChangeKind::Code),
        other => anyhow::bail!("unknown kind '{other}'; use config, profile, identity, or code"),
    }
}
