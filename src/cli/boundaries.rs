use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Subcommand};

use crate::{
    boundaries::{
        BoundaryReviewState, add_boundary, confirm_review, list_boundaries, review_prompt,
    },
    cli::core::load_initialized_config,
    time::{Clock, SystemClock},
};

#[derive(Debug, Args)]
pub struct BoundariesArgs {
    #[command(subcommand)]
    pub command: BoundariesCommand,
}

#[derive(Debug, Subcommand)]
pub enum BoundariesCommand {
    Show,
    Add(BoundariesAddArgs),
    Confirm(BoundariesConfirmArgs),
}

#[derive(Debug, Args)]
pub struct BoundariesAddArgs {
    #[arg(required = true)]
    pub rule: Vec<String>,
}

#[derive(Debug, Args)]
pub struct BoundariesConfirmArgs {
    #[arg(long)]
    pub note: Option<String>,
}

pub(crate) fn handle_boundaries(
    data_dir_override: Option<PathBuf>,
    args: BoundariesArgs,
) -> Result<String> {
    match args.command {
        BoundariesCommand::Show => show(data_dir_override),
        BoundariesCommand::Add(args) => add(data_dir_override, args),
        BoundariesCommand::Confirm(args) => confirm(data_dir_override, args),
    }
}

fn show(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let now = SystemClock::from_env()?.now_utc();
    let state = BoundaryReviewState::load_or_default(&paths.boundary_review_file)?;
    let mut lines = vec![format!("boundary_review_due: {}", state.review_due(now))];
    if let Some(prompt) = review_prompt(&state, now) {
        lines.push(format!("boundary_review_prompt: {prompt}"));
    }
    if let Some(confirmed) = state.last_confirmed_at {
        lines.push(format!("last_boundary_review: {confirmed}"));
    }
    let rules = list_boundaries(&paths.identity_file)?;
    if rules.is_empty() {
        lines.push("boundaries: none explicitly recorded".to_string());
    } else {
        lines.push("boundaries:".to_string());
        lines.extend(rules.into_iter().map(|rule| format!("- {rule}")));
    }
    Ok(lines.join("\n"))
}

fn add(data_dir_override: Option<PathBuf>, args: BoundariesAddArgs) -> Result<String> {
    let rule = args.rule.join(" ").trim().to_string();
    if rule.is_empty() {
        bail!("boundary rule must not be empty");
    }
    let (_, paths) = load_initialized_config(data_dir_override)?;
    add_boundary(&paths.identity_file, &rule)?;
    Ok(format!("boundary: added\nrule: {rule}"))
}

fn confirm(data_dir_override: Option<PathBuf>, args: BoundariesConfirmArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let now = SystemClock::from_env()?.now_utc();
    let state = confirm_review(&paths.boundary_review_file, now, args.note.as_deref())?;
    Ok(format!(
        "boundary_review_confirmed: {}\nnote: {}",
        state.last_confirmed_at.unwrap_or_default(),
        state.last_note.unwrap_or_else(|| "-".to_string())
    ))
}
