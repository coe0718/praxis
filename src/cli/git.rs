use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};

use crate::paths::{PraxisPaths, default_data_dir};

#[derive(Debug, Args)]
pub struct GitArgs {
    #[command(subcommand)]
    command: GitCommand,
}

#[derive(Debug, Subcommand)]
enum GitCommand {
    /// Initialize a git repository in the Praxis data directory.
    Init,
    /// Stage all changes and commit with an auto-generated message.
    Commit(GitCommitArgs),
    /// Commit and push to the configured remote.
    Push(GitPushArgs),
    /// Show git status of the data directory.
    Status,
}

#[derive(Debug, Args)]
struct GitCommitArgs {
    /// Commit message (auto-generated if omitted).
    #[arg(long, short)]
    message: Option<String>,
}

#[derive(Debug, Args)]
struct GitPushArgs {
    /// Remote name (default: origin).
    #[arg(long, default_value = "origin")]
    remote: String,

    /// Branch name (default: main).
    #[arg(long, default_value = "main")]
    branch: String,

    /// Commit message (auto-generated if omitted).
    #[arg(long, short)]
    message: Option<String>,
}

const GITIGNORE: &str = "\
# Praxis — secrets and ephemeral state never go into git
master.key
oauth_tokens.json
security.toml
*.lock
";

pub(crate) fn handle_git(data_dir_override: Option<PathBuf>, args: GitArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        GitCommand::Init => git_init(&paths),
        GitCommand::Commit(a) => git_commit(&paths, a.message.as_deref()),
        GitCommand::Push(a) => git_push(&paths, &a.remote, &a.branch, a.message.as_deref()),
        GitCommand::Status => git_status(&paths),
    }
}

fn git_init(paths: &PraxisPaths) -> Result<String> {
    let dir = &paths.data_dir;

    let gitignore = dir.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, GITIGNORE)
            .with_context(|| format!("failed to write {}", gitignore.display()))?;
    }

    if dir.join(".git").exists() {
        return Ok(format!("git: already initialized in {}", dir.display()));
    }

    run_git(dir, &["init"])?;
    run_git(dir, &["add", ".gitignore"])?;
    run_git(
        dir,
        &[
            "commit",
            "--allow-empty",
            "-m",
            "chore: initialize praxis data repo",
        ],
    )?;

    Ok(format!("git: initialized in {}", dir.display()))
}

fn git_commit(paths: &PraxisPaths, message: Option<&str>) -> Result<String> {
    ensure_git_repo(&paths.data_dir)?;
    let msg = message
        .map(str::to_string)
        .unwrap_or_else(auto_commit_message);
    run_git(&paths.data_dir, &["add", "-A"])?;
    let result = run_git(&paths.data_dir, &["commit", "-m", &msg]);
    match result {
        Ok(out) if out.contains("nothing to commit") || out.is_empty() => {
            Ok("git: nothing to commit".to_string())
        }
        Ok(_) => Ok(format!("git: committed\nmessage: {msg}")),
        Err(e) => {
            // "nothing to commit" surfaces as a non-zero exit on some git versions.
            if e.to_string().contains("nothing to commit") {
                Ok("git: nothing to commit".to_string())
            } else {
                Err(e)
            }
        }
    }
}

fn git_push(
    paths: &PraxisPaths,
    remote: &str,
    branch: &str,
    message: Option<&str>,
) -> Result<String> {
    ensure_git_repo(&paths.data_dir)?;
    let msg = message
        .map(str::to_string)
        .unwrap_or_else(auto_commit_message);
    run_git(&paths.data_dir, &["add", "-A"])?;
    // Commit only if there's something staged; ignore "nothing to commit".
    let commit_out = Command::new("git")
        .args(["commit", "-m", &msg])
        .current_dir(&paths.data_dir)
        .output()
        .context("failed to run git commit")?;
    let committed = commit_out.status.success();

    run_git(&paths.data_dir, &["push", remote, branch])?;

    if committed {
        Ok(format!(
            "git: committed and pushed to {remote}/{branch}\nmessage: {msg}"
        ))
    } else {
        Ok(format!("git: pushed to {remote}/{branch} (no new commit)"))
    }
}

fn git_status(paths: &PraxisPaths) -> Result<String> {
    ensure_git_repo(&paths.data_dir)?;
    let out = run_git(&paths.data_dir, &["status", "--short"])?;
    if out.trim().is_empty() {
        Ok("git: clean — nothing to commit".to_string())
    } else {
        Ok(format!("git status:\n{}", out.trim()))
    }
}

fn ensure_git_repo(dir: &Path) -> Result<()> {
    if !dir.join(".git").exists() {
        bail!(
            "{} is not a git repository — run `praxis git init` first",
            dir.display()
        );
    }
    Ok(())
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        let detail = if stderr.is_empty() { &stdout } else { &stderr };
        bail!("git {} failed: {}", args.join(" "), detail);
    }

    Ok(stdout)
}

fn auto_commit_message() -> String {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    format!("chore: praxis state sync {now}")
}

/// Silently commit all state changes in `data_dir` if it is a git repository.
///
/// Called automatically at the end of every Reflect phase.  Non-fatal: any
/// git failure (nothing to commit, no remote, etc.) is silently ignored.
pub(crate) fn auto_commit(paths: &PraxisPaths) {
    if !paths.data_dir.join(".git").exists() {
        return;
    }
    let _ = run_git(&paths.data_dir, &["add", "-A"]);
    let _ = run_git(&paths.data_dir, &["commit", "-m", &auto_commit_message()]);
}
