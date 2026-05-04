//! Skills CLI — list, search, install, update skills from local and remote catalogs.  (#4)

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::{Args, Subcommand};

use crate::cli::core::load_initialized_config;
use crate::skills::{self, DEFAULT_REGISTRY_URL};

#[derive(Debug, Args)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Subcommand)]
pub enum SkillsCommand {
    /// List locally-installed skills.
    List,

    /// Search the remote registry for skills matching a query.
    Search {
        /// Search query (matches name, description, tags).
        query: Vec<String>,

        /// Registry URL (default: agentskills.io).
        #[arg(long)]
        registry: Option<String>,
    },

    /// Install a skill from a URL or by name from the remote registry.
    Install {
        /// Skill ID (to search registry) or a direct URL to a SKILL.md file.
        source: String,

        /// Registry URL (default: agentskills.io).
        #[arg(long)]
        registry: Option<String>,
    },

    /// Update all locally-installed skills from the remote registry.
    Update {
        /// Registry URL (default: agentskills.io).
        #[arg(long)]
        registry: Option<String>,
    },
}

pub fn handle_skills(data_dir_override: Option<PathBuf>, args: SkillsArgs) -> Result<String> {
    match args.command {
        SkillsCommand::List => handle_list(data_dir_override),
        SkillsCommand::Search { query, registry } => {
            handle_search(data_dir_override, &query.join(" "), registry)
        }
        SkillsCommand::Install { source, registry } => {
            handle_install(data_dir_override, &source, registry)
        }
        SkillsCommand::Update { registry } => handle_update(data_dir_override, registry),
    }
}

fn handle_list(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let catalog = skills::load_catalog(&paths.skills_dir);

    if catalog.is_empty() {
        return Ok("No skills installed.".to_string());
    }

    let mut out = format!("Installed skills ({}):\n", catalog.len());
    for entry in &catalog {
        out.push_str(&entry.catalog_line());
        out.push('\n');
    }
    Ok(out)
}

fn handle_search(
    data_dir_override: Option<PathBuf>,
    query: &str,
    registry_url: Option<String>,
) -> Result<String> {
    // Ensure config is loaded (validates data_dir exists).
    let (_, _paths) = load_initialized_config(data_dir_override)?;

    let url = registry_url.as_deref().unwrap_or(DEFAULT_REGISTRY_URL);
    let catalog = skills::fetch_remote_catalog(url)
        .map_err(|e| anyhow!("Failed to fetch remote catalog from {url}: {e:#}"))?;

    let results = skills::search_remote_catalog(&catalog, query);

    if results.is_empty() {
        return Ok(format!("No remote skills matching '{query}'."));
    }

    let mut out = format!("Remote skills matching '{query}' ({}):\n", results.len());
    for entry in &results {
        out.push_str(&entry.catalog_line());
        out.push('\n');
    }
    Ok(out)
}

fn handle_install(
    data_dir_override: Option<PathBuf>,
    source: &str,
    registry_url: Option<String>,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;

    // If source looks like a URL, install directly.
    if source.starts_with("http://") || source.starts_with("https://") {
        let id = extract_id_from_url(source);
        let dest = skills::install_skill_from_url(&paths.skills_dir, &id, source)?;
        return Ok(format!("Installed skill '{id}' from URL → {}", dest.display()));
    }

    // Otherwise, search the remote registry and install the first match.
    let url = registry_url.as_deref().unwrap_or(DEFAULT_REGISTRY_URL);
    let catalog = skills::fetch_remote_catalog(url)
        .map_err(|e| anyhow!("Failed to fetch remote catalog from {url}: {e:#}"))?;

    let matches = skills::search_remote_catalog(&catalog, source);
    let entry = matches.into_iter().next().ok_or_else(|| {
        anyhow!("No remote skill matching '{source}'. Use a direct URL or check the skill ID.")
    })?;

    let dest = skills::install_skill_from_url(&paths.skills_dir, &entry.id, &entry.url)?;

    Ok(format!("Installed skill '{}' ({}) → {}", entry.name, entry.id, dest.display()))
}

fn handle_update(
    data_dir_override: Option<PathBuf>,
    registry_url: Option<String>,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let url = registry_url.as_deref().unwrap_or(DEFAULT_REGISTRY_URL);

    let (updated, failed) = skills::update_skills_from_registry(&paths.skills_dir, url)?;

    Ok(format!("Skills updated: {updated} updated, {failed} failed."))
}

/// Extract a skill ID from a URL by taking the filename stem.
fn extract_id_from_url(url: &str) -> String {
    url.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(".md")
        .to_string()
}
