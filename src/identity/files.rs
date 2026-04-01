use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::{config::AppConfig, paths::PraxisPaths, storage::StoredSession};

use super::IdentityPolicy;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalIdentityPolicy;

impl IdentityPolicy for LocalIdentityPolicy {
    fn ensure_foundation(
        &self,
        paths: &PraxisPaths,
        config: &AppConfig,
        now: DateTime<Utc>,
    ) -> Result<()> {
        fs::create_dir_all(&paths.data_dir)
            .with_context(|| format!("failed to create {}", paths.data_dir.display()))?;
        fs::create_dir_all(&paths.goals_criteria_dir)
            .with_context(|| format!("failed to create {}", paths.goals_criteria_dir.display()))?;
        fs::create_dir_all(&paths.evals_dir)
            .with_context(|| format!("failed to create {}", paths.evals_dir.display()))?;

        for (path, contents) in foundation_documents(paths, config, now) {
            write_if_missing(&path, contents)?;
        }

        Ok(())
    }

    fn validate(&self, paths: &PraxisPaths) -> Result<()> {
        for path in paths.identity_files() {
            if !path.exists() {
                bail!("missing required identity file {}", path.display());
            }
        }

        if self.read_day_count(paths)? < 0 {
            bail!("DAY_COUNT must not be negative");
        }

        Ok(())
    }

    fn append_journal(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()> {
        let mut file = appendable_file(&paths.journal_file)?;
        writeln!(file)?;
        writeln!(file, "## {}", session.ended_at)?;
        writeln!(file, "- Outcome: {}", session.outcome)?;

        if let Some(goal_id) = &session.selected_goal_id {
            let title = session
                .selected_goal_title
                .as_deref()
                .unwrap_or("unknown goal");
            writeln!(file, "- Goal: {goal_id}: {title}")?;
        }

        if let Some(task) = &session.selected_task {
            writeln!(file, "- Task: {task}")?;
        }

        writeln!(file, "- Summary: {}", session.action_summary)?;
        Ok(())
    }

    fn append_metrics(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()> {
        let mut file = appendable_file(&paths.metrics_file)?;
        writeln!(
            file,
            "| {} | {} | {} | {} | {} |",
            session.session_num,
            sanitize_markdown_cell(&session.outcome),
            sanitize_markdown_cell(session.selected_goal_title.as_deref().unwrap_or("-")),
            sanitize_markdown_cell(session.selected_task.as_deref().unwrap_or("-")),
            sanitize_markdown_cell(&session.ended_at),
        )?;
        Ok(())
    }

    fn read_day_count(&self, paths: &PraxisPaths) -> Result<i64> {
        let raw = fs::read_to_string(&paths.day_count_file)
            .with_context(|| format!("failed to read {}", paths.day_count_file.display()))?;
        raw.trim().parse::<i64>().with_context(|| {
            format!(
                "invalid DAY_COUNT value in {}",
                paths.day_count_file.display()
            )
        })
    }
}

fn foundation_documents(
    paths: &PraxisPaths,
    config: &AppConfig,
    now: DateTime<Utc>,
) -> Vec<(std::path::PathBuf, String)> {
    vec![
        (
            paths.identity_file.clone(),
            format!(
                "# Identity\n\n- Name: {}\n- Timezone: {}\n- Role: A local Praxis instance in foundation mode.\n",
                config.instance.name, config.instance.timezone
            ),
        ),
        (
            paths.goals_file.clone(),
            "# Goals\n\n- [ ] G-001: Complete the first Praxis foundation run\n".to_string(),
        ),
        (
            paths.roadmap_file.clone(),
            "# Roadmap\n\n- Foundation milestone in progress.\n".to_string(),
        ),
        (
            paths.journal_file.clone(),
            format!(
                "# Journal\n\n## Day 0\n- Initialized Praxis foundation on {}.\n",
                now.to_rfc3339()
            ),
        ),
        (
            paths.metrics_file.clone(),
            "# Metrics\n\n| Session | Outcome | Goal | Task | Ended At |\n| --- | --- | --- | --- | --- |\n"
                .to_string(),
        ),
        (
            paths.patterns_file.clone(),
            "# Patterns\n\n- No patterns captured yet.\n".to_string(),
        ),
        (
            paths.learnings_file.clone(),
            "# Learnings\n\n- No learnings captured yet.\n".to_string(),
        ),
        (
            paths.capabilities_file.clone(),
            "# Capabilities\n\n- CLI foundation\n- SQLite session storage\n- Resumable loop state\n"
                .to_string(),
        ),
        (
            paths.proposals_file.clone(),
            "# Proposals\n\n- No pending proposals.\n".to_string(),
        ),
        (paths.day_count_file.clone(), "0\n".to_string()),
        (
            paths.goals_criteria_dir.join("G-001.json"),
            r#"{
  "goal_id": "G-001",
  "done_when": [
    "Core Praxis foundation files exist",
    "The local SQLite database exists",
    "Default tool manifests are present"
  ],
  "verify_with": "shell",
  "commands": [
    "test -f IDENTITY.md",
    "test -f praxis.db",
    "test -f tools/praxis-data-write.toml"
  ]
}
"#
            .to_string(),
        ),
        (
            paths.evals_dir.join("foundation-smoke.json"),
            r#"{
  "id": "foundation-smoke",
  "name": "Foundation runtime files stay healthy",
  "when": "always",
  "severity": "functional",
  "scenario": "After a session, Praxis should still have the minimum local foundation files it needs.",
  "expected_behavior": [
    "Identity and metrics files are present",
    "The SQLite database remains available"
  ],
  "forbidden_behavior": [
    "Core foundation files disappear during a session"
  ],
  "relevant_memories": [
    "foundation milestone"
  ],
  "verify_with": "shell",
  "commands": [
    "test -f IDENTITY.md",
    "test -f METRICS.md",
    "test -f praxis.db"
  ]
}
"#
            .to_string(),
        ),
    ]
}

fn appendable_file(path: &Path) -> Result<fs::File> {
    OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))
}

fn write_if_missing(path: &Path, contents: String) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn sanitize_markdown_cell(value: &str) -> String {
    value.replace('|', "/")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::LocalIdentityPolicy;
    use crate::{config::AppConfig, identity::IdentityPolicy, paths::PraxisPaths};

    #[test]
    fn validates_identity_files() {
        let temp = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(temp.path().join("praxis"));
        let config = AppConfig::default_for_data_dir(paths.data_dir.clone());
        let policy = LocalIdentityPolicy;
        let now = chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap();

        policy.ensure_foundation(&paths, &config, now).unwrap();
        policy.validate(&paths).unwrap();
    }
}
