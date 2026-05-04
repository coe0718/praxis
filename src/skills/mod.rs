//! Progressive skill loading and synthesis.
//!
//! Skills live as `*.md` files in the `skills/` directory.  Each file has a
//! TOML frontmatter block between `+++` delimiters at the top, followed by the
//! full skill body.
//!
//! Example:
//! ```text
//! +++
//! name = "morning-brief"
//! description = "Collect and push a morning brief to Telegram"
//! tags = ["telegram", "brief", "daily"]
//! token_estimate = 800
//! +++
//!
//! ## Steps
//! 1. Gather memory and goal summaries …
//! ```
//!
//! Orient loads only the compact catalog (name + description + tags +
//! token_estimate) to keep the context cost low.  Full content is pulled
//! on-demand when a skill is actually needed in a plan.
//!
//! # Remote Registry (#4)
//!
//! Skills can be fetched from a remote registry (agentskills.io or any
//! compatible HTTP endpoint).  The catalog is a JSON array of skill metadata.
//! Individual skills can be installed from URL.

pub mod synthesis;

pub use synthesis::SkillSynthesizer;

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Compact metadata about a single skill.
#[derive(Debug, Clone)]
pub struct SkillEntry {
    /// Stable identifier derived from the filename stem.
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    /// Rough token count of the full skill body.
    pub token_estimate: usize,
}

impl SkillEntry {
    /// Render as a one-liner for the compact catalog.
    pub fn catalog_line(&self) -> String {
        let tags = if self.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.tags.join(", "))
        };
        format!(
            "- {} — {}{}  (~{} tokens)",
            self.name, self.description, tags, self.token_estimate
        )
    }
}

/// Compact metadata stored in the TOML frontmatter of a `SKILL.md` file.
#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default = "default_token_estimate")]
    token_estimate: usize,
}

fn default_token_estimate() -> usize {
    500
}

/// Load the compact skill catalog from `skills_dir`.
/// Files that cannot be parsed are silently skipped so a malformed draft does
/// not break Orient.
pub fn load_catalog(skills_dir: &Path) -> Vec<SkillEntry> {
    let Ok(entries) = fs::read_dir(skills_dir) else {
        return Vec::new();
    };

    let mut catalog: Vec<SkillEntry> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? != "md" {
                return None;
            }
            let id = path.file_stem()?.to_string_lossy().into_owned();
            let raw = fs::read_to_string(&path).ok()?;
            let fm = parse_frontmatter(&raw)?;
            Some(SkillEntry {
                id,
                name: fm.name,
                description: fm.description,
                tags: fm.tags,
                token_estimate: fm.token_estimate,
            })
        })
        .collect();

    catalog.sort_by(|a, b| a.name.cmp(&b.name));
    catalog
}

/// Render the compact skill catalog as a string for inclusion in the context.
/// Returns an empty string when no skills are installed.
pub fn render_catalog(skills_dir: &Path) -> String {
    let catalog = load_catalog(skills_dir);
    if catalog.is_empty() {
        return String::new();
    }
    let mut out = String::from("## Installed Skills\n");
    for entry in &catalog {
        out.push_str(&entry.catalog_line());
        out.push('\n');
    }
    out
}

/// Read the full body of a skill by id.  Returns `None` if the file is absent
/// or cannot be read.
pub fn read_skill_content(skills_dir: &Path, skill_id: &str) -> Option<String> {
    let path = skills_dir.join(format!("{skill_id}.md"));
    let raw = fs::read_to_string(path).ok()?;
    Some(strip_frontmatter(&raw))
}

fn parse_frontmatter(raw: &str) -> Option<SkillFrontmatter> {
    let body = raw.trim_start();
    if !body.starts_with("+++") {
        return None;
    }
    let rest = &body[3..];
    let end = rest.find("+++")?;
    let toml_src = &rest[..end];
    toml::from_str(toml_src).ok()
}

fn strip_frontmatter(raw: &str) -> String {
    let body = raw.trim_start();
    if !body.starts_with("+++") {
        return raw.to_string();
    }
    let rest = &body[3..];
    if let Some(end) = rest.find("+++") {
        rest[end + 3..].trim_start().to_string()
    } else {
        raw.to_string()
    }
}

// ── Remote Registry (#4) ────────────────────────────────────────

/// Default remote skill registry URL.
pub const DEFAULT_REGISTRY_URL: &str = "https://agentskills.io/api/v1/catalog";

/// A single entry in the remote catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteSkillEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub token_estimate: usize,
    /// URL to download the full SKILL.md content.
    pub url: String,
}

impl RemoteSkillEntry {
    pub fn catalog_line(&self) -> String {
        let tags = if self.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.tags.join(", "))
        };
        format!(
            "- {} — {}{}  (~{} tokens)",
            self.name, self.description, tags, self.token_estimate
        )
    }
}

/// Fetch the remote skill catalog from a registry URL.
///
/// The endpoint must return a JSON array of `RemoteSkillEntry` objects.
/// Returns an empty vec on network errors (graceful degradation).
pub fn fetch_remote_catalog(registry_url: &str) -> Result<Vec<RemoteSkillEntry>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("praxis-skills/1.0")
        .build()
        .context("building HTTP client for skill registry")?;

    let resp = client.get(registry_url).send().context("fetching remote skill catalog")?;

    if !resp.status().is_success() {
        bail!("registry returned HTTP {}", resp.status());
    }

    let catalog: Vec<RemoteSkillEntry> = resp.json().context("parsing remote catalog JSON")?;

    Ok(catalog)
}

/// Search the remote catalog for skills matching a query.
///
/// Matches against name, description, and tags (case-insensitive).
pub fn search_remote_catalog<'a>(
    catalog: &'a [RemoteSkillEntry],
    query: &str,
) -> Vec<&'a RemoteSkillEntry> {
    let q = query.to_lowercase();
    catalog
        .iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&q)
                || entry.description.to_lowercase().contains(&q)
                || entry.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .collect()
}

/// Install a skill from a remote URL into the local skills directory.
///
/// Downloads the full SKILL.md content and saves it to `skills_dir/<id>.md`.
/// If the skill already exists locally, it is overwritten.
pub fn install_skill_from_url(skills_dir: &Path, id: &str, url: &str) -> Result<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("praxis-skills/1.0")
        .build()
        .context("building HTTP client for skill install")?;

    let resp = client
        .get(url)
        .send()
        .with_context(|| format!("downloading skill from {url}"))?;

    if !resp.status().is_success() {
        bail!("skill download returned HTTP {}", resp.status());
    }

    let content = resp.text().context("reading skill content")?;

    // Validate it looks like a skill (has frontmatter or is markdown).
    if !content.contains("+++") && !content.starts_with('#') {
        bail!(
            "downloaded content does not appear to be a valid skill (missing frontmatter or markdown header)"
        );
    }

    fs::create_dir_all(skills_dir)
        .with_context(|| format!("creating skills dir {}", skills_dir.display()))?;

    let dest = skills_dir.join(format!("{id}.md"));
    fs::write(&dest, &content).with_context(|| format!("writing skill to {}", dest.display()))?;

    Ok(dest)
}

/// Update all locally-installed skills that have a known remote source.
///
/// Reads the local catalog, and for each skill that has a `remote_url`
/// embedded in a comment, re-downloads it. Returns (updated, failed) counts.
pub fn update_skills_from_registry(
    skills_dir: &Path,
    registry_url: &str,
) -> Result<(usize, usize)> {
    let remote = fetch_remote_catalog(registry_url)?;
    let local = load_catalog(skills_dir);
    let mut updated = 0usize;
    let mut failed = 0usize;

    for local_skill in &local {
        if let Some(remote_match) = remote.iter().find(|r| r.id == local_skill.id) {
            match install_skill_from_url(skills_dir, &remote_match.id, &remote_match.url) {
                Ok(_) => updated += 1,
                Err(e) => {
                    log::warn!("failed to update skill {}: {e:#}", local_skill.id);
                    failed += 1;
                }
            }
        }
    }

    Ok((updated, failed))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn write_skill(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join(format!("{name}.md")), body).unwrap();
    }

    #[test]
    fn loads_catalog_from_skills_dir() {
        let tmp = tempdir().unwrap();
        write_skill(
            tmp.path(),
            "morning-brief",
            "+++\nname = \"morning-brief\"\ndescription = \"Push a morning brief\"\ntags = [\"telegram\", \"daily\"]\ntoken_estimate = 600\n+++\n\n## Steps\n- Gather context\n",
        );
        write_skill(
            tmp.path(),
            "code-review",
            "+++\nname = \"code-review\"\ndescription = \"Review a PR\"\n+++\n\n## Steps\n- Read diff\n",
        );

        let catalog = load_catalog(tmp.path());
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog[0].id, "code-review");
        assert_eq!(catalog[1].token_estimate, 600);
    }

    #[test]
    fn renders_catalog_as_compact_lines() {
        let tmp = tempdir().unwrap();
        write_skill(
            tmp.path(),
            "deploy",
            "+++\nname = \"deploy\"\ndescription = \"Deploy to production\"\ntags = [\"ops\"]\ntoken_estimate = 300\n+++\n",
        );

        let rendered = render_catalog(tmp.path());
        assert!(rendered.contains("## Installed Skills"));
        assert!(rendered.contains("deploy"));
        assert!(rendered.contains("~300 tokens"));
    }

    #[test]
    fn reads_full_skill_content_by_id() {
        let tmp = tempdir().unwrap();
        write_skill(
            tmp.path(),
            "triage",
            "+++\nname = \"triage\"\ndescription = \"Triage backlog\"\n+++\n\n## Instructions\nStep one.\n",
        );

        let content = read_skill_content(tmp.path(), "triage").unwrap();
        assert!(content.contains("## Instructions"));
        assert!(!content.contains("+++"));
    }

    #[test]
    fn catalog_is_empty_when_dir_absent() {
        let catalog = load_catalog(Path::new("/nonexistent/skills"));
        assert!(catalog.is_empty());
    }

    #[test]
    fn search_remote_catalog_matches_name_and_tags() {
        let entries = vec![
            RemoteSkillEntry {
                id: "deploy".into(),
                name: "deploy".into(),
                description: "Deploy to production".into(),
                tags: vec!["ops".into()],
                token_estimate: 300,
                url: "https://example.com/deploy.md".into(),
            },
            RemoteSkillEntry {
                id: "code-review".into(),
                name: "code-review".into(),
                description: "Review pull requests".into(),
                tags: vec!["github".into()],
                token_estimate: 400,
                url: "https://example.com/review.md".into(),
            },
        ];

        let results = search_remote_catalog(&entries, "ops");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "deploy");

        let results = search_remote_catalog(&entries, "pull");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "code-review");

        let results = search_remote_catalog(&entries, "github");
        assert_eq!(results.len(), 1);
    }
}
