use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use crate::paths::PraxisPaths;

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct AgentsArgs {
    #[command(subcommand)]
    pub command: AgentsCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentsCommand {
    View,
    Add(AgentsAddArgs),
}

#[derive(Debug, Args)]
pub struct AgentsAddArgs {
    #[arg(long, value_enum, default_value_t = AgentsSection::Workflow)]
    pub section: AgentsSection,

    #[arg(long, required = true)]
    pub note: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AgentsSection {
    Workflow,
    Gotcha,
    Handoff,
}

impl AgentsSection {
    fn heading(self) -> &'static str {
        match self {
            Self::Workflow => "## Workflow Notes",
            Self::Gotcha => "## Gotchas",
            Self::Handoff => "## Handoffs",
        }
    }
}

pub(crate) fn handle_agents(
    data_dir_override: Option<PathBuf>,
    args: AgentsArgs,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    match args.command {
        AgentsCommand::View => fs::read_to_string(&paths.agents_file)
            .with_context(|| format!("failed to read {}", paths.agents_file.display())),
        AgentsCommand::Add(args) => add_note(&paths, args),
    }
}

fn add_note(paths: &PraxisPaths, args: AgentsAddArgs) -> Result<String> {
    let note = args.note.join(" ").trim().to_string();
    if note.is_empty() {
        bail!("agent note must not be empty");
    }

    let content = fs::read_to_string(&paths.agents_file)
        .with_context(|| format!("failed to read {}", paths.agents_file.display()))?;
    let updated = insert_under_heading(&content, args.section.heading(), &note);
    fs::write(&paths.agents_file, updated)
        .with_context(|| format!("failed to write {}", paths.agents_file.display()))?;

    Ok(format!(
        "agents: updated\nsection: {}\nnote: {}",
        args.section.heading().trim_start_matches("## "),
        note
    ))
}

fn insert_under_heading(content: &str, heading: &str, note: &str) -> String {
    let mut lines = content.lines().map(str::to_string).collect::<Vec<_>>();
    let Some(index) = lines.iter().position(|line| line.trim() == heading) else {
        let mut updated = content.trim_end().to_string();
        updated.push_str(&format!("\n\n{heading}\n- {note}\n"));
        return updated;
    };
    let insert_at = lines[index + 1..]
        .iter()
        .position(|line| line.starts_with("## "))
        .map(|offset| index + 1 + offset)
        .unwrap_or(lines.len());
    lines.insert(insert_at, format!("- {note}"));
    lines.join("\n") + "\n"
}
