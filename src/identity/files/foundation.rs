use chrono::{DateTime, Utc};

use crate::{config::AppConfig, paths::PraxisPaths};

pub(super) fn foundation_documents(
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
            "# Proposals\n\n## Pending\n- No pending proposals.\n\n## Accepted\n- No accepted proposals.\n\n## Dismissed\n- No dismissed proposals.\n".to_string(),
        ),
        (paths.day_count_file.clone(), "0\n".to_string()),
        (
            paths.learning_sources_dir.join("operator-notes.md"),
            "# Operator Notes\n\n- Capture new preferences and useful recurring habits here.\n"
                .to_string(),
        ),
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
