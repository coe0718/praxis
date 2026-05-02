//! Worktree-based git isolation for agent sessions.
//!
//! Creates a separate git worktree per session or task so that concurrent
//! operations don't step on each other's changes. Each worktree gets its
//! own branch and working directory, with optional auto-cleanup.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

/// A managed git worktree for isolated work.
#[derive(Debug)]
pub struct Worktree {
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// Branch name used in this worktree.
    pub branch: String,
    /// The original repo path.
    pub repo_root: PathBuf,
}

/// Create a new worktree for isolated work.
///
/// `repo_root` must point to a git repository (or a subdirectory of one).
/// A new branch `branch_prefix/session-id` is created from HEAD, and a
/// worktree is checked out at `repo_root/.praxis-worktrees/<name>`.
pub fn create_worktree(repo_root: &Path, name: &str, branch_prefix: &str) -> Result<Worktree> {
    let git_dir = find_git_root(repo_root)?;
    let branch_name = format!("{branch_prefix}/{name}");
    let worktrees_dir = git_dir.join(".praxis-worktrees");
    fs::create_dir_all(&worktrees_dir)
        .with_context(|| format!("creating worktrees dir {}", worktrees_dir.display()))?;

    let worktree_path = worktrees_dir.join(name);

    if worktree_path.exists() {
        bail!(
            "worktree already exists at {} — remove it first or use a different name",
            worktree_path.display()
        );
    }

    // Create a new branch from HEAD
    let branch_output = git_cmd(&git_dir, &["branch", &branch_name])?;
    if !branch_output.status.success() {
        let stderr = String::from_utf8_lossy(&branch_output.stderr);
        // Branch might already exist — try checkout anyway
        log::warn!("branch create warning: {stderr}");
    }

    // Create worktree at the target path, checking out the new branch
    let output = git_cmd(
        &git_dir,
        &["worktree", "add", worktree_path.to_str().unwrap_or_default(), &branch_name],
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Cleanup the branch we just created
        let _ = git_cmd(&git_dir, &["branch", "-D", &branch_name]);
        bail!("git worktree add failed: {stderr}");
    }

    Ok(Worktree {
        path: worktree_path,
        branch: branch_name,
        repo_root: git_dir,
    })
}

/// List all praxis-managed worktrees in a repo.
pub fn list_worktrees(repo_root: &Path) -> Result<Vec<Worktree>> {
    let git_dir = find_git_root(repo_root)?;
    let worktrees_dir = git_dir.join(".praxis-worktrees");

    if !worktrees_dir.exists() {
        return Ok(Vec::new());
    }

    let mut worktrees = Vec::new();
    let entries = fs::read_dir(&worktrees_dir)
        .with_context(|| format!("reading {}", worktrees_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Read the branch from the worktree's HEAD
            let head_file = path.join(".git");
            let name_str = name_from_path(&path);
            let branch = if let Ok(ref_content) = fs::read_to_string(&head_file) {
                // Worktree .git is a file pointing to the actual .git
                parse_branch_from_gitdir(&git_dir, &ref_content).unwrap_or(name_str.clone())
            } else {
                name_str.clone()
            };

            worktrees.push(Worktree {
                path,
                branch,
                repo_root: git_dir.clone(),
            });
        }
    }

    Ok(worktrees)
}

/// Remove a worktree and optionally delete its branch.
pub fn remove_worktree(repo_root: &Path, name: &str, delete_branch: bool) -> Result<String> {
    let git_dir = find_git_root(repo_root)?;
    let worktree_path = git_dir.join(".praxis-worktrees").join(name);

    if !worktree_path.exists() {
        bail!("worktree '{}' not found", name);
    }

    // Remove the worktree
    let output = git_cmd(
        &git_dir,
        &["worktree", "remove", worktree_path.to_str().unwrap_or_default(), "--force"],
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git worktree remove failed: {stderr}");
    }

    let branch_name = format!("praxis/{name}");
    let mut deleted_branch = false;
    if delete_branch {
        let branch_output = git_cmd(&git_dir, &["branch", "-D", &branch_name]);
        deleted_branch = branch_output.map(|o| o.status.success()).unwrap_or(false);
    }

    // Prune worktree metadata
    let _ = git_cmd(&git_dir, &["worktree", "prune"]);

    Ok(format!(
        "removed worktree '{}'{}",
        name,
        if deleted_branch { " (branch deleted)" } else { "" }
    ))
}

/// Merge a worktree's branch back into the main branch.
pub fn merge_worktree(repo_root: &Path, name: &str, target_branch: &str) -> Result<String> {
    let git_dir = find_git_root(repo_root)?;
    let branch_name = format!("praxis/{name}");

    // Check the branch exists
    let check = git_cmd(&git_dir, &["rev-parse", "--verify", &branch_name])?;
    if !check.status.success() {
        bail!("branch '{}' does not exist", branch_name);
    }

    // Checkout target, merge, return to previous branch
    let current = git_cmd(&git_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let current_branch = String::from_utf8_lossy(&current.stdout).trim().to_string();

    // Switch to target
    let checkout = git_cmd(&git_dir, &["checkout", target_branch])?;
    if !checkout.status.success() {
        bail!("failed to checkout '{}'", target_branch);
    }

    // Merge
    let merge = git_cmd(&git_dir, &["merge", &branch_name, "--no-edit"])?;
    if !merge.status.success() {
        // Abort the merge on failure
        let _ = git_cmd(&git_dir, &["merge", "--abort"]);
        let _ = git_cmd(&git_dir, &["checkout", &current_branch]);
        let stderr = String::from_utf8_lossy(&merge.stderr);
        bail!("merge failed (aborted): {stderr}");
    }

    // Switch back to original branch
    if current_branch != target_branch {
        let _ = git_cmd(&git_dir, &["checkout", &current_branch]);
    }

    Ok(format!(
        "merged '{}' into '{}' (back on '{}')",
        branch_name, target_branch, current_branch
    ))
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn git_cmd(dir: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| format!("running git {} in {}", args.join(" "), dir.display()))
}

fn find_git_root(path: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("finding git root")?;

    if !output.status.success() {
        bail!("{} is not inside a git repository", path.display());
    }

    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(root))
}

fn name_from_path(path: &Path) -> String {
    path.file_name().unwrap_or_default().to_string_lossy().to_string()
}

fn parse_branch_from_gitdir(git_dir: &Path, gitdir_content: &str) -> Option<String> {
    // gitdir content is like: "git: /path/to/.git/worktrees/name"
    // The actual HEAD is in .git/worktrees/name/HEAD
    let worktree_name = gitdir_content.trim().rsplit('/').next().unwrap_or("").to_string();
    if worktree_name.is_empty() {
        return None;
    }
    Some(format!("praxis/{worktree_name}"))
}

// ── CLI Handler ──────────────────────────────────────────────────────────

/// CLI handler for `praxis worktree`.
pub fn handle_worktree(data_dir: Option<PathBuf>, args: super::WorktreeArgs) -> Result<String> {
    use crate::paths::{PraxisPaths, default_data_dir};

    let data_dir_path = data_dir.unwrap_or(default_data_dir()?);
    let _paths = PraxisPaths::for_data_dir(data_dir_path);

    // Determine repo root from config or CWD
    let repo_root = std::env::current_dir().context("getting current directory")?;

    match args.command {
        super::WorktreeCommand::Create(create_args) => {
            let name = create_args.name.clone();
            let wt = create_worktree(&repo_root, &name, "praxis")?;
            Ok(format!(
                "created worktree '{}' at {}\n  branch: {}",
                name,
                wt.path.display(),
                wt.branch
            ))
        }
        super::WorktreeCommand::List => {
            let worktrees = list_worktrees(&repo_root)?;
            if worktrees.is_empty() {
                return Ok("no praxis worktrees found.".to_string());
            }
            let mut lines = vec![format!("found {} worktree(s):", worktrees.len())];
            for wt in &worktrees {
                let name = wt.path.file_name().unwrap_or_default().to_string_lossy();
                lines.push(format!("  {} → {} ({})", name, wt.path.display(), wt.branch));
            }
            Ok(lines.join("\n"))
        }
        super::WorktreeCommand::Remove(remove_args) => {
            remove_worktree(&repo_root, &remove_args.name, remove_args.delete_branch)
        }
        super::WorktreeCommand::Merge(merge_args) => {
            let target = merge_args.target_branch.clone();
            merge_worktree(&repo_root, &merge_args.name, &target)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_remove_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();

        // Init a git repo
        git_cmd(repo, &["init"]).unwrap();
        git_cmd(repo, &["config", "user.email", "test@test.com"]).unwrap();
        git_cmd(repo, &["config", "user.name", "Test"]).unwrap();
        fs::write(repo.join("README.md"), "# Test").unwrap();
        git_cmd(repo, &["add", "."]).unwrap();
        git_cmd(repo, &["commit", "-m", "initial"]).unwrap();

        let wt = create_worktree(repo, "test-session", "praxis").unwrap();
        assert!(wt.path.exists());
        assert_eq!(wt.branch, "praxis/test-session");

        let result = remove_worktree(repo, "test-session", true).unwrap();
        assert!(result.contains("removed worktree"));
    }

    #[test]
    fn test_list_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path();
        git_cmd(repo, &["init"]).unwrap();

        let worktrees = list_worktrees(repo).unwrap();
        assert!(worktrees.is_empty());
    }
}
