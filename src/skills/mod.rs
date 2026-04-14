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

pub mod synthesis;

pub use synthesis::SkillSynthesizer;

use std::{fs, path::Path};

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
}
