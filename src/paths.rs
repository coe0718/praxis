use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOs,
    Other,
}

#[derive(Debug, Clone)]
pub struct PraxisPaths {
    pub data_dir: PathBuf,
    pub config_file: PathBuf,
    pub providers_file: PathBuf,
    pub profiles_file: PathBuf,
    pub budgets_file: PathBuf,
    pub database_file: PathBuf,
    pub state_file: PathBuf,
    /// Immutable core identity anchor — operator-only, never agent-writable.
    pub soul_file: PathBuf,
    /// Evolving working identity written at foundation and updated by the agent.
    pub identity_file: PathBuf,
    /// Structured dialectic operator model (working hypotheses, confirmed traits,
    /// tensions, counter-evidence) — freely writable by the agent.
    pub operator_model_file: PathBuf,
    pub goals_file: PathBuf,
    pub roadmap_file: PathBuf,
    pub journal_file: PathBuf,
    pub metrics_file: PathBuf,
    pub patterns_file: PathBuf,
    pub learnings_file: PathBuf,
    pub agents_file: PathBuf,
    pub capabilities_file: PathBuf,
    pub proposals_file: PathBuf,
    pub day_count_file: PathBuf,
    pub events_file: PathBuf,
    pub heartbeat_file: PathBuf,
    pub context_adaptation_file: PathBuf,
    pub model_canary_file: PathBuf,
    pub boundary_review_file: PathBuf,
    pub telegram_state_file: PathBuf,
    pub learning_dir: PathBuf,
    pub learning_sources_dir: PathBuf,
    pub backups_dir: PathBuf,
    pub evals_dir: PathBuf,
    pub goals_dir: PathBuf,
    pub goals_criteria_dir: PathBuf,
    pub tools_dir: PathBuf,
    /// Installed skills — each `*.md` file is a `SKILL.md` document.
    pub skills_dir: PathBuf,
    /// Draft skills awaiting operator review before activation.
    pub skills_drafts_dir: PathBuf,
    /// Capability benchmark cases — JSON files in `benchmarks/`.
    pub benchmarks_dir: PathBuf,
    /// Append-only log of benchmark run results (`benchmark_log.jsonl`).
    pub benchmark_log_file: PathBuf,
    /// Central message bus — inbound events from all adapters (`bus.jsonl`).
    pub bus_file: PathBuf,
    /// Per-conversation activation modes (`activation.json`).
    pub activation_file: PathBuf,
    /// Sensitive config overrides separated from the committable `praxis.toml` (`security.toml`).
    pub security_file: PathBuf,
    /// Sender pairing state — approved + pending unknown-chat requests (`sender_pairing.json`).
    pub sender_pairing_file: PathBuf,
    /// Auto-generated conflict workbench (`MEMORY_CONFLICTS.md`).
    pub memory_conflicts_file: PathBuf,
    /// Persistent operator memory across sessions (`user_memory.json`).
    pub user_memory_file: PathBuf,
    /// Per-session task decomposition (`todo.json`).
    pub todo_file: PathBuf,
    /// Compressed working-set cache from the last Reflect phase (`context_cache.json`).
    pub context_cache_file: PathBuf,
    /// Tool-level approval cooldown state (`tool_cooldowns.json`).
    pub tool_cooldowns_file: PathBuf,
    /// Per-conversation context isolation state (`context_groups.json`).
    pub context_groups_file: PathBuf,
    /// Operator interaction histogram for adaptive scheduling (`operator_schedule.json`).
    pub operator_schedule_file: PathBuf,
    /// System resource anomaly log (`system_anomalies.jsonl`).
    pub system_anomalies_file: PathBuf,
    /// Configured inter-agent delegation links (`delegation_links.json`).
    pub delegation_links_file: PathBuf,
    /// Inbound delegation task queue written by remote agents (`delegation_queue.jsonl`).
    pub delegation_queue_file: PathBuf,
    /// Installed hand manifests — each `*.toml` describes a role bundle (`hands/`).
    pub hands_dir: PathBuf,
    /// Name of the currently active hand (`hands/active.txt`). Empty file = no active hand.
    pub active_hand_file: PathBuf,
    /// Per-session irreplaceability score log (`score.jsonl`).
    pub score_file: PathBuf,
    /// Credential vault — name→secret mapping resolved at request time (`vault.toml`).
    pub vault_file: PathBuf,
    /// Daemon PID file — written when the daemon starts, removed on clean exit.
    pub daemon_pid_file: PathBuf,
    /// Hook definitions — shell scripts bound to runtime events (`hooks.toml`).
    pub hooks_file: PathBuf,
    /// Per-channel sandbox policies (`channel_sandbox.json`).
    pub sandbox_file: PathBuf,
    /// Canary freeze state — provider/model pairs frozen by automation (`canary_frozen.json`).
    pub canary_freeze_file: PathBuf,
    /// Dynamic route weight overrides written by canary automation (`canary_weights.json`).
    pub route_weights_file: PathBuf,
    /// Meta-evolution proposal log (`evolution.jsonl`).
    pub evolution_file: PathBuf,
    /// Human-readable evolution doc regenerated from the log (`SELF_EVOLUTION.md`).
    pub self_evolution_file: PathBuf,
    /// Discord polling state — last seen message IDs per channel (`discord_state.json`).
    pub discord_state_file: PathBuf,
    /// Slack polling state — last seen message timestamps per channel (`slack_state.json`).
    pub slack_state_file: PathBuf,
    /// Watchdog self-update state — pending/applied binary update records (`watchdog_update.json`).
    pub watchdog_update_file: PathBuf,
    /// AES-256 master key for at-rest encryption of vault and OAuth tokens (`master.key`).
    /// Generated once on `praxis init`; must be excluded from version control.
    pub master_key_file: PathBuf,
    /// Agent-callable scheduled jobs (`scheduled_jobs.json`).
    pub scheduled_jobs_file: PathBuf,
}

pub fn detect_platform() -> Platform {
    if cfg!(target_os = "linux") {
        Platform::Linux
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Other
    }
}

pub fn default_data_dir() -> Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set, unable to resolve a default Praxis data directory")?;
    let xdg_data_home = env::var_os("XDG_DATA_HOME").map(PathBuf::from);

    Ok(default_data_dir_for(detect_platform(), &home, xdg_data_home.as_deref()))
}

pub fn default_data_dir_for(
    platform: Platform,
    home: &Path,
    xdg_data_home: Option<&Path>,
) -> PathBuf {
    match platform {
        Platform::Linux => xdg_data_home
            .map(|dir| dir.join("praxis"))
            .unwrap_or_else(|| home.join(".local/share/praxis")),
        Platform::MacOs => home.join("Library/Application Support/Praxis"),
        Platform::Other => home.join(".praxis"),
    }
}

impl PraxisPaths {
    pub fn for_data_dir(data_dir: PathBuf) -> Self {
        Self::new(data_dir, PathBuf::from("praxis.db"), PathBuf::from("session_state.json"))
    }

    pub fn from_config(config: &AppConfig) -> Self {
        Self::new(
            config.instance.data_dir.clone(),
            config.database.path.clone(),
            config.runtime.state_file.clone(),
        )
    }

    pub fn new(data_dir: PathBuf, database_path: PathBuf, state_file: PathBuf) -> Self {
        let goals_dir = data_dir.join("goals");
        let goals_criteria_dir = goals_dir.join("criteria");
        let evals_dir = data_dir.join("evals");
        let tools_dir = data_dir.join("tools");
        let skills_dir = data_dir.join("skills");
        let skills_drafts_dir = skills_dir.join("drafts");
        let benchmarks_dir = data_dir.join("benchmarks");
        let learning_dir = data_dir.join("learning");
        let learning_sources_dir = learning_dir.join("sources");
        let backups_dir = data_dir.join("backups");
        let hands_dir = data_dir.join("hands");

        Self {
            config_file: data_dir.join("praxis.toml"),
            providers_file: data_dir.join("providers.toml"),
            profiles_file: data_dir.join("profiles.toml"),
            budgets_file: data_dir.join("budgets.toml"),
            database_file: resolve_path(&data_dir, &database_path),
            state_file: resolve_path(&data_dir, &state_file),
            soul_file: data_dir.join("SOUL.md"),
            identity_file: data_dir.join("IDENTITY.md"),
            operator_model_file: data_dir.join("operator_model.json"),
            goals_file: data_dir.join("GOALS.md"),
            roadmap_file: data_dir.join("ROADMAP.md"),
            journal_file: data_dir.join("JOURNAL.md"),
            metrics_file: data_dir.join("METRICS.md"),
            patterns_file: data_dir.join("PATTERNS.md"),
            learnings_file: data_dir.join("LEARNINGS.md"),
            agents_file: data_dir.join("AGENTS.md"),
            capabilities_file: data_dir.join("CAPABILITIES.md"),
            proposals_file: data_dir.join("PROPOSALS.md"),
            day_count_file: data_dir.join("DAY_COUNT"),
            events_file: data_dir.join("events.jsonl"),
            heartbeat_file: data_dir.join("heartbeat.json"),
            context_adaptation_file: data_dir.join("context_adaptation.json"),
            model_canary_file: data_dir.join("model_canary.json"),
            boundary_review_file: data_dir.join("boundary_review.json"),
            telegram_state_file: data_dir.join("telegram_state.json"),
            learning_dir,
            learning_sources_dir,
            backups_dir,
            evals_dir,
            goals_dir,
            goals_criteria_dir,
            tools_dir,
            skills_dir,
            skills_drafts_dir,
            benchmark_log_file: data_dir.join("benchmark_log.jsonl"),
            bus_file: data_dir.join("bus.jsonl"),
            activation_file: data_dir.join("activation.json"),
            security_file: data_dir.join("security.toml"),
            sender_pairing_file: data_dir.join("sender_pairing.json"),
            memory_conflicts_file: data_dir.join("MEMORY_CONFLICTS.md"),
            user_memory_file: data_dir.join("user_memory.json"),
            todo_file: data_dir.join("todo.json"),
            context_cache_file: data_dir.join("context_cache.json"),
            tool_cooldowns_file: data_dir.join("tool_cooldowns.json"),
            context_groups_file: data_dir.join("context_groups.json"),
            operator_schedule_file: data_dir.join("operator_schedule.json"),
            system_anomalies_file: data_dir.join("system_anomalies.jsonl"),
            delegation_links_file: data_dir.join("delegation_links.json"),
            delegation_queue_file: data_dir.join("delegation_queue.jsonl"),
            active_hand_file: data_dir.join("hands").join("active.txt"),
            hands_dir,
            score_file: data_dir.join("score.jsonl"),
            vault_file: data_dir.join("vault.toml"),
            daemon_pid_file: data_dir.join("daemon.pid"),
            hooks_file: data_dir.join("hooks.toml"),
            sandbox_file: data_dir.join("channel_sandbox.json"),
            canary_freeze_file: data_dir.join("canary_frozen.json"),
            route_weights_file: data_dir.join("canary_weights.json"),
            evolution_file: data_dir.join("evolution.jsonl"),
            self_evolution_file: data_dir.join("SELF_EVOLUTION.md"),
            discord_state_file: data_dir.join("discord_state.json"),
            slack_state_file: data_dir.join("slack_state.json"),
            watchdog_update_file: data_dir.join("watchdog_update.json"),
            master_key_file: data_dir.join("master.key"),
            scheduled_jobs_file: data_dir.join("scheduled_jobs.json"),
            benchmarks_dir,
            data_dir,
        }
    }

    pub fn identity_files(&self) -> Vec<PathBuf> {
        vec![
            self.soul_file.clone(),
            self.identity_file.clone(),
            self.goals_file.clone(),
            self.roadmap_file.clone(),
            self.journal_file.clone(),
            self.metrics_file.clone(),
            self.patterns_file.clone(),
            self.learnings_file.clone(),
            self.agents_file.clone(),
            self.capabilities_file.clone(),
            self.proposals_file.clone(),
            self.day_count_file.clone(),
        ]
    }
}

fn resolve_path(base: &Path, candidate: &Path) -> PathBuf {
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Platform, default_data_dir_for};

    #[test]
    fn linux_prefers_xdg_data_home() {
        let home = PathBuf::from("/home/tester");
        let xdg = PathBuf::from("/tmp/data-home");

        let path = default_data_dir_for(Platform::Linux, &home, Some(&xdg));
        assert_eq!(path, PathBuf::from("/tmp/data-home/praxis"));
    }

    #[test]
    fn macos_uses_application_support() {
        let home = PathBuf::from("/Users/tester");
        let path = default_data_dir_for(Platform::MacOs, &home, None);

        assert_eq!(path, PathBuf::from("/Users/tester/Library/Application Support/Praxis"));
    }
}
