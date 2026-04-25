//! Progressive context file loader — walks from the current working directory
//! up to the git repository root, collecting per-directory context files that
//! influence agent behavior in specific subdirectories.
//!
//! Discovered files (in priority order):
//!   `.praxis.md`    — Praxis-specific per-directory conventions
//!   `AGENTS.md`     — Project-level agent guidance
//!   `CLAUDE.md`     — Claude Code working guide
//!   `.cursorrules`  — Cursor editor rules
//!
//! Files from deeper directories override / extend those from parent dirs.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

/// Ordered list of filenames to discover, highest priority first.
const PROGRESSIVE_FILES: &[&str] = &[".praxis.md", "AGENTS.md", "CLAUDE.md", ".cursorrules"];

/// Result of a progressive context scan.
#[derive(Debug, Clone)]
pub struct ProgressiveContext {
    /// Map of directory path → concatenated file contents from that directory.
    pub entries: Vec<ProgressiveEntry>,
}

#[derive(Debug, Clone)]
pub struct ProgressiveEntry {
    /// Relative directory path from the repository root.
    pub dir: String,
    /// Filenames found in this directory.
    pub files: Vec<String>,
    /// Combined content from all found files.
    pub content: String,
}

/// Scan from `cwd` upward to the git root, collecting progressive context files
/// at each directory level. Returns entries ordered from root to cwd (deepest last).
pub fn load_progressive_context(cwd: &Path) -> Result<ProgressiveContext> {
    let git_root = find_git_root(cwd);
    let root = git_root.as_deref().unwrap_or(cwd);

    // Walk from root to cwd, collecting files at each level.
    let mut entries = Vec::new();

    // Collect all directory levels from root to cwd.
    let mut dirs = Vec::new();
    let mut current = cwd.to_path_buf();
    loop {
        dirs.push(current.clone());
        if current == root {
            break;
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }
    dirs.reverse(); // root first, cwd last

    for dir in &dirs {
        let mut files = Vec::new();
        let mut content = String::new();

        for filename in PROGRESSIVE_FILES {
            let file_path = dir.join(filename);
            if file_path.is_file() {
                if let Ok(raw) = fs::read_to_string(&file_path) {
                    if !content.is_empty() {
                        content.push_str("\n\n");
                    }
                    content.push_str(&format!("--- {} ---\n{}", filename, raw.trim()));
                    files.push(filename.to_string());
                }
            }
        }

        if !files.is_empty() {
            let rel_dir = if dir == root {
                ".".to_string()
            } else {
                dir.strip_prefix(root).unwrap_or(dir).display().to_string()
            };

            entries.push(ProgressiveEntry { dir: rel_dir, files, content });
        }
    }

    Ok(ProgressiveContext { entries })
}

/// Find the nearest git repository root by walking up from `start`.
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").is_dir() {
            return Some(current);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

impl ProgressiveContext {
    /// Render all progressive context entries as a single markdown block
    /// suitable for injection into the agent's system prompt.
    pub fn render(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        let mut out = String::from("## Progressive Context\n\n");
        out.push_str("Per-directory conventions discovered from the working tree:\n\n");

        for entry in &self.entries {
            out.push_str(&format!("### {}/\n", entry.dir));
            out.push_str(&format!("Files: {}\n\n", entry.files.join(", ")));
            out.push_str(&entry.content);
            out.push_str("\n\n");
        }

        out.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn discovers_files_at_multiple_levels() {
        let tmp = tempdir().unwrap();
        let git_root = tmp.path().join("repo");
        fs::create_dir_all(git_root.join(".git")).unwrap();
        fs::create_dir_all(git_root.join("subdir")).unwrap();

        fs::write(git_root.join("AGENTS.md"), "# Root agents\nBe careful.\n").unwrap();
        fs::write(git_root.join("subdir").join(".praxis.md"), "# Subdir rules\nRun fast.\n")
            .unwrap();

        let ctx = load_progressive_context(&git_root.join("subdir")).unwrap();
        assert_eq!(ctx.entries.len(), 2);
        assert_eq!(ctx.entries[0].dir, ".");
        assert!(ctx.entries[0].content.contains("Root agents"));
        assert_eq!(ctx.entries[1].dir, "subdir");
        assert!(ctx.entries[1].content.contains("Run fast"));
    }

    #[test]
    fn no_files_returns_empty() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();

        let ctx = load_progressive_context(tmp.path()).unwrap();
        assert!(ctx.entries.is_empty());
        assert!(ctx.render().is_empty());
    }

    #[test]
    fn renders_context_block() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        fs::write(tmp.path().join(".praxis.md"), "# Praxis config\nUse Rust.\n").unwrap();

        let ctx = load_progressive_context(tmp.path()).unwrap();
        let rendered = ctx.render();
        assert!(rendered.contains("## Progressive Context"));
        assert!(rendered.contains("### ./"));
        assert!(rendered.contains("Use Rust"));
    }

    #[test]
    fn no_git_root_falls_back_to_cwd() {
        let tmp = tempdir().unwrap();
        // No .git dir — should still scan CWD.
        fs::write(tmp.path().join("AGENTS.md"), "# Agents\nTest mode.\n").unwrap();

        let ctx = load_progressive_context(tmp.path()).unwrap();
        assert_eq!(ctx.entries.len(), 1);
        assert!(ctx.entries[0].content.contains("Test mode"));
    }
}
