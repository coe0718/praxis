//! Lite mode — reduces sub-agent usage, tightens budgets, and simplifies
//! behaviour for Raspberry Pi or low-power installs.
//!
//! When `praxis.toml` contains `[agent] lite = true`:
//! - context ceiling drops to 60% (from 80%)
//! - no speculative execution branches
//! - no sub-agent reviewers (self-review only, logged)
//! - no synthetic example generation
//! - no daily learning runs
//! - no morning brief generation
//! - no autonomous curator skill grading cycles
//! - reduced anatomy refresh frequency
//! - disabled dashboard SSE stream
//!
//! Lite mode is about staying functional on constrained hardware, not about
//! cutting safety.  Approval queues, loop guards, and file-mutation circuit
//! breakers all remain active.

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

/// Lite-mode configuration persisted in `praxis.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LiteMode {
    pub enabled: bool,
    #[serde(default = "default_context_ceiling_pct")]
    pub context_ceiling_pct: f32,
    #[serde(default = "default_disable_speculative")]
    pub disable_speculative: bool,
    #[serde(default = "default_disable_subagent_reviewers")]
    pub disable_subagent_reviewers: bool,
    #[serde(default = "default_disable_synthetic_examples")]
    pub disable_synthetic_examples: bool,
    #[serde(default = "default_disable_learning")]
    pub disable_learning: bool,
    #[serde(default = "default_disable_brief")]
    pub disable_brief: bool,
    #[serde(default = "default_anatomy_refresh_hours")]
    pub anatomy_refresh_hours: u64,
    #[serde(default = "default_disable_sse")]
    pub disable_sse: bool,
    #[serde(default = "default_disable_deterministic")]
    pub disable_deterministic: bool,
    #[serde(default = "default_disable_curator")]
    pub disable_curator: bool,
    #[serde(default = "default_disable_federation")]
    pub disable_federation: bool,
    #[serde(default = "default_disable_openmolt")]
    pub disable_openmolt: bool,
    #[serde(default = "default_disable_wave")]
    pub disable_wave: bool,
    #[serde(default = "default_disable_channels")]
    pub disable_channels: bool,
    #[serde(default = "default_disable_hotreload")]
    pub disable_hotreload: bool,
    // Wave 2 — HIGH priority wiring
    #[serde(default = "default_disable_injection")]
    pub disable_injection: bool,
    #[serde(default = "default_disable_leaks")]
    pub disable_leaks: bool,
    #[serde(default = "default_disable_tracing")]
    pub disable_tracing: bool,
    #[serde(default = "default_disable_docker_isolation")]
    pub disable_docker_isolation: bool,
    #[serde(default = "default_disable_sandbox_enforcement")]
    pub disable_sandbox_enforcement: bool,
    #[serde(default = "default_disable_checkpoints")]
    pub disable_checkpoints: bool,
    #[serde(default = "default_disable_rules")]
    pub disable_rules: bool,
    #[serde(default = "default_disable_routines")]
    pub disable_routines: bool,
    #[serde(default = "default_disable_voice")]
    pub disable_voice: bool,
    #[serde(default = "default_disable_self_update")]
    pub disable_self_update: bool,
    // Wave 3 — MEDIUM priority wiring
    #[serde(default = "default_disable_embedding_cache")]
    pub disable_embedding_cache: bool,
    #[serde(default = "default_disable_rrf")]
    pub disable_rrf: bool,
    #[serde(default = "default_disable_archive")]
    pub disable_archive: bool,
    #[serde(default = "default_disable_backup")]
    pub disable_backup: bool,
    #[serde(default = "default_disable_runtime_skill")]
    pub disable_runtime_skill: bool,
    #[serde(default = "default_disable_trigger")]
    pub disable_trigger: bool,
    #[serde(default = "default_disable_observability")]
    pub disable_observability: bool,
    #[serde(default = "default_disable_merkle")]
    pub disable_merkle: bool,
    #[serde(default = "default_disable_tool_schema")]
    pub disable_tool_schema: bool,
    #[serde(default = "default_disable_canvas")]
    pub disable_canvas: bool,
    #[serde(default = "default_disable_carapace")]
    pub disable_carapace: bool,
    #[serde(default = "default_disable_attachments")]
    pub disable_attachments: bool,
    // Wave 4 — LOW priority wiring
    #[serde(default = "default_disable_bench")]
    pub disable_bench: bool,
    #[serde(default = "default_disable_browser")]
    pub disable_browser: bool,
    #[serde(default = "default_disable_browser_pwa")]
    pub disable_browser_pwa: bool,
    #[serde(default = "default_disable_gitclaw")]
    pub disable_gitclaw: bool,
    #[serde(default = "default_disable_zeptoclaw")]
    pub disable_zeptoclaw: bool,
    #[serde(default = "default_disable_zh_channels")]
    pub disable_zh_channels: bool,
    #[serde(default = "default_disable_onchain_reputation")]
    pub disable_onchain_reputation: bool,
    #[serde(default = "default_disable_rating_improve")]
    pub disable_rating_improve: bool,
    #[serde(default = "default_disable_skill_pack")]
    pub disable_skill_pack: bool,
    #[serde(default = "default_disable_i18n")]
    pub disable_i18n: bool,
}

impl Default for LiteMode {
    fn default() -> Self {
        Self {
            enabled: false,
            context_ceiling_pct: default_context_ceiling_pct(),
            disable_speculative: default_disable_speculative(),
            disable_subagent_reviewers: default_disable_subagent_reviewers(),
            disable_synthetic_examples: default_disable_synthetic_examples(),
            disable_learning: default_disable_learning(),
            disable_brief: default_disable_brief(),
            anatomy_refresh_hours: default_anatomy_refresh_hours(),
            disable_deterministic: default_disable_deterministic(),
            disable_sse: default_disable_sse(),
            disable_curator: default_disable_curator(),
            disable_federation: default_disable_federation(),
            disable_openmolt: default_disable_openmolt(),
            disable_wave: default_disable_wave(),
            disable_channels: default_disable_channels(),
            disable_hotreload: default_disable_hotreload(),
            // HIGH
            disable_injection: default_disable_injection(),
            disable_leaks: default_disable_leaks(),
            disable_tracing: default_disable_tracing(),
            disable_docker_isolation: default_disable_docker_isolation(),
            disable_sandbox_enforcement: default_disable_sandbox_enforcement(),
            disable_checkpoints: default_disable_checkpoints(),
            disable_rules: default_disable_rules(),
            disable_routines: default_disable_routines(),
            disable_voice: default_disable_voice(),
            disable_self_update: default_disable_self_update(),
            // MEDIUM
            disable_embedding_cache: default_disable_embedding_cache(),
            disable_rrf: default_disable_rrf(),
            disable_archive: default_disable_archive(),
            disable_backup: default_disable_backup(),
            disable_runtime_skill: default_disable_runtime_skill(),
            disable_trigger: default_disable_trigger(),
            disable_observability: default_disable_observability(),
            disable_merkle: default_disable_merkle(),
            disable_tool_schema: default_disable_tool_schema(),
            disable_canvas: default_disable_canvas(),
            disable_carapace: default_disable_carapace(),
            disable_attachments: default_disable_attachments(),
            // LOW
            disable_bench: default_disable_bench(),
            disable_browser: default_disable_browser(),
            disable_browser_pwa: default_disable_browser_pwa(),
            disable_gitclaw: default_disable_gitclaw(),
            disable_zeptoclaw: default_disable_zeptoclaw(),
            disable_zh_channels: default_disable_zh_channels(),
            disable_onchain_reputation: default_disable_onchain_reputation(),
            disable_rating_improve: default_disable_rating_improve(),
            disable_skill_pack: default_disable_skill_pack(),
            disable_i18n: default_disable_i18n(),
        }
    }
}

fn default_context_ceiling_pct() -> f32 {
    0.60
}
fn default_disable_speculative() -> bool {
    true
}
fn default_disable_subagent_reviewers() -> bool {
    true
}
fn default_disable_synthetic_examples() -> bool {
    true
}
fn default_disable_learning() -> bool {
    true
}
fn default_disable_brief() -> bool {
    true
}
fn default_anatomy_refresh_hours() -> u64 {
    48
}
fn default_disable_sse() -> bool {
    true
}
fn default_disable_deterministic() -> bool {
    false
}
fn default_disable_curator() -> bool {
    false
}
fn default_disable_federation() -> bool {
    true
}
fn default_disable_openmolt() -> bool {
    true
}
fn default_disable_wave() -> bool {
    false
}
fn default_disable_channels() -> bool {
    false
}
fn default_disable_hotreload() -> bool {
    true
}
// HIGH — security features default to OFF (enabled) so they run in normal mode
fn default_disable_injection() -> bool {
    false
}
fn default_disable_leaks() -> bool {
    false
}
fn default_disable_tracing() -> bool {
    false
}
fn default_disable_docker_isolation() -> bool {
    true
}
fn default_disable_sandbox_enforcement() -> bool {
    false
}
fn default_disable_checkpoints() -> bool {
    true
}
fn default_disable_rules() -> bool {
    true
}
fn default_disable_routines() -> bool {
    true
}
fn default_disable_voice() -> bool {
    true
}
fn default_disable_self_update() -> bool {
    true
}
// MEDIUM
fn default_disable_embedding_cache() -> bool {
    true
}
fn default_disable_rrf() -> bool {
    true
}
fn default_disable_archive() -> bool {
    true
}
fn default_disable_backup() -> bool {
    true
}
fn default_disable_runtime_skill() -> bool {
    true
}
fn default_disable_trigger() -> bool {
    true
}
fn default_disable_observability() -> bool {
    true
}
fn default_disable_merkle() -> bool {
    true
}
fn default_disable_tool_schema() -> bool {
    true
}
fn default_disable_canvas() -> bool {
    true
}
fn default_disable_carapace() -> bool {
    true
}
fn default_disable_attachments() -> bool {
    true
}
// LOW
fn default_disable_bench() -> bool {
    true
}
fn default_disable_browser() -> bool {
    true
}
fn default_disable_browser_pwa() -> bool {
    true
}
fn default_disable_gitclaw() -> bool {
    true
}
fn default_disable_zeptoclaw() -> bool {
    true
}
fn default_disable_zh_channels() -> bool {
    true
}
fn default_disable_onchain_reputation() -> bool {
    true
}
fn default_disable_rating_improve() -> bool {
    true
}
fn default_disable_skill_pack() -> bool {
    true
}
fn default_disable_i18n() -> bool {
    true
}

impl LiteMode {
    /// Load lite mode settings from a `praxis.toml` file.
    /// Returns defaults when the file is missing or has no `[agent]` section.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
        let config: toml::Value = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid TOML in {}: {e}", path.display()))?;
        let agent = config.get("agent").and_then(|v| v.as_table());
        let enabled = agent
            .and_then(|t| t.get("lite"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);

        let mut lite = Self { enabled, ..Self::default() };
        if let Some(t) = agent {
            if let Some(v) = t.get("lite_context_ceiling_pct").and_then(toml::Value::as_float) {
                lite.context_ceiling_pct = v as f32;
            }
            if let Some(v) = t.get("lite_disable_speculative").and_then(toml::Value::as_bool) {
                lite.disable_speculative = v;
            }
            if let Some(v) = t.get("lite_disable_subagent_reviewers").and_then(toml::Value::as_bool)
            {
                lite.disable_subagent_reviewers = v;
            }
            if let Some(v) = t.get("lite_disable_synthetic_examples").and_then(toml::Value::as_bool)
            {
                lite.disable_synthetic_examples = v;
            }
            if let Some(v) = t.get("lite_disable_learning").and_then(toml::Value::as_bool) {
                lite.disable_learning = v;
            }
            if let Some(v) = t.get("lite_disable_brief").and_then(toml::Value::as_bool) {
                lite.disable_brief = v;
            }
            if let Some(v) = t.get("lite_anatomy_refresh_hours").and_then(toml::Value::as_integer) {
                lite.anatomy_refresh_hours = v as u64;
            }
            if let Some(v) = t.get("lite_disable_sse").and_then(toml::Value::as_bool) {
                lite.disable_sse = v;
            }
            if let Some(v) = t.get("lite_disable_curator").and_then(toml::Value::as_bool) {
                lite.disable_curator = v;
            }
            if let Some(v) = t.get("lite_disable_federation").and_then(toml::Value::as_bool) {
                lite.disable_federation = v;
            }
            if let Some(v) = t.get("lite_disable_openmolt").and_then(toml::Value::as_bool) {
                lite.disable_openmolt = v;
            }
            if let Some(v) = t.get("lite_disable_wave").and_then(toml::Value::as_bool) {
                lite.disable_wave = v;
            }
            if let Some(v) = t.get("lite_disable_channels").and_then(toml::Value::as_bool) {
                lite.disable_channels = v;
            }
            if let Some(v) = t.get("lite_disable_hotreload").and_then(toml::Value::as_bool) {
                lite.disable_hotreload = v;
            }
            // HIGH
            if let Some(v) = t.get("lite_disable_injection").and_then(toml::Value::as_bool) {
                lite.disable_injection = v;
            }
            if let Some(v) = t.get("lite_disable_leaks").and_then(toml::Value::as_bool) {
                lite.disable_leaks = v;
            }
            if let Some(v) = t.get("lite_disable_tracing").and_then(toml::Value::as_bool) {
                lite.disable_tracing = v;
            }
            if let Some(v) = t.get("lite_disable_docker_isolation").and_then(toml::Value::as_bool) {
                lite.disable_docker_isolation = v;
            }
            if let Some(v) =
                t.get("lite_disable_sandbox_enforcement").and_then(toml::Value::as_bool)
            {
                lite.disable_sandbox_enforcement = v;
            }
            if let Some(v) = t.get("lite_disable_checkpoints").and_then(toml::Value::as_bool) {
                lite.disable_checkpoints = v;
            }
            if let Some(v) = t.get("lite_disable_rules").and_then(toml::Value::as_bool) {
                lite.disable_rules = v;
            }
            if let Some(v) = t.get("lite_disable_routines").and_then(toml::Value::as_bool) {
                lite.disable_routines = v;
            }
            if let Some(v) = t.get("lite_disable_voice").and_then(toml::Value::as_bool) {
                lite.disable_voice = v;
            }
            if let Some(v) = t.get("lite_disable_self_update").and_then(toml::Value::as_bool) {
                lite.disable_self_update = v;
            }
            // MEDIUM
            if let Some(v) = t.get("lite_disable_embedding_cache").and_then(toml::Value::as_bool) {
                lite.disable_embedding_cache = v;
            }
            if let Some(v) = t.get("lite_disable_rrf").and_then(toml::Value::as_bool) {
                lite.disable_rrf = v;
            }
            if let Some(v) = t.get("lite_disable_archive").and_then(toml::Value::as_bool) {
                lite.disable_archive = v;
            }
            if let Some(v) = t.get("lite_disable_backup").and_then(toml::Value::as_bool) {
                lite.disable_backup = v;
            }
            if let Some(v) = t.get("lite_disable_runtime_skill").and_then(toml::Value::as_bool) {
                lite.disable_runtime_skill = v;
            }
            if let Some(v) = t.get("lite_disable_trigger").and_then(toml::Value::as_bool) {
                lite.disable_trigger = v;
            }
            if let Some(v) = t.get("lite_disable_observability").and_then(toml::Value::as_bool) {
                lite.disable_observability = v;
            }
            if let Some(v) = t.get("lite_disable_merkle").and_then(toml::Value::as_bool) {
                lite.disable_merkle = v;
            }
            if let Some(v) = t.get("lite_disable_tool_schema").and_then(toml::Value::as_bool) {
                lite.disable_tool_schema = v;
            }
            if let Some(v) = t.get("lite_disable_canvas").and_then(toml::Value::as_bool) {
                lite.disable_canvas = v;
            }
            if let Some(v) = t.get("lite_disable_carapace").and_then(toml::Value::as_bool) {
                lite.disable_carapace = v;
            }
            if let Some(v) = t.get("lite_disable_attachments").and_then(toml::Value::as_bool) {
                lite.disable_attachments = v;
            }
            // LOW
            if let Some(v) = t.get("lite_disable_bench").and_then(toml::Value::as_bool) {
                lite.disable_bench = v;
            }
            if let Some(v) = t.get("lite_disable_browser").and_then(toml::Value::as_bool) {
                lite.disable_browser = v;
            }
            if let Some(v) = t.get("lite_disable_browser_pwa").and_then(toml::Value::as_bool) {
                lite.disable_browser_pwa = v;
            }
            if let Some(v) = t.get("lite_disable_gitclaw").and_then(toml::Value::as_bool) {
                lite.disable_gitclaw = v;
            }
            if let Some(v) = t.get("lite_disable_zeptoclaw").and_then(toml::Value::as_bool) {
                lite.disable_zeptoclaw = v;
            }
            if let Some(v) = t.get("lite_disable_zh_channels").and_then(toml::Value::as_bool) {
                lite.disable_zh_channels = v;
            }
            if let Some(v) = t.get("lite_disable_onchain_reputation").and_then(toml::Value::as_bool)
            {
                lite.disable_onchain_reputation = v;
            }
            if let Some(v) = t.get("lite_disable_rating_improve").and_then(toml::Value::as_bool) {
                lite.disable_rating_improve = v;
            }
            if let Some(v) = t.get("lite_disable_skill_pack").and_then(toml::Value::as_bool) {
                lite.disable_skill_pack = v;
            }
            if let Some(v) = t.get("lite_disable_i18n").and_then(toml::Value::as_bool) {
                lite.disable_i18n = v;
            }
        }
        Ok(lite)
    }

    /// Returns `true` if a capability should be skipped in lite mode.
    pub fn skip_capability(&self, cap: LiteCapability) -> bool {
        if !self.enabled {
            return false;
        }
        match cap {
            LiteCapability::Speculative => self.disable_speculative,
            LiteCapability::SubagentReviewer => self.disable_subagent_reviewers,
            LiteCapability::SyntheticExamples => self.disable_synthetic_examples,
            LiteCapability::Learning => self.disable_learning,
            LiteCapability::Brief => self.disable_brief,
            LiteCapability::SseStream => self.disable_sse,
            LiteCapability::Deterministic => self.disable_deterministic,
            LiteCapability::Curator => self.disable_curator,
            LiteCapability::Federation => self.disable_federation,
            LiteCapability::OpenMolt => self.disable_openmolt,
            LiteCapability::Wave => self.disable_wave,
            LiteCapability::Channels => self.disable_channels,
            LiteCapability::HotReload => self.disable_hotreload,
            // HIGH
            LiteCapability::Injection => self.disable_injection,
            LiteCapability::Leaks => self.disable_leaks,
            LiteCapability::Tracing => self.disable_tracing,
            LiteCapability::DockerIsolation => self.disable_docker_isolation,
            LiteCapability::SandboxEnforcement => self.disable_sandbox_enforcement,
            LiteCapability::Checkpoints => self.disable_checkpoints,
            LiteCapability::Rules => self.disable_rules,
            LiteCapability::Routines => self.disable_routines,
            LiteCapability::Voice => self.disable_voice,
            LiteCapability::SelfUpdate => self.disable_self_update,
            // MEDIUM
            LiteCapability::EmbeddingCache => self.disable_embedding_cache,
            LiteCapability::Rrf => self.disable_rrf,
            LiteCapability::Archive => self.disable_archive,
            LiteCapability::Backup => self.disable_backup,
            LiteCapability::RuntimeSkill => self.disable_runtime_skill,
            LiteCapability::Trigger => self.disable_trigger,
            LiteCapability::Observability => self.disable_observability,
            LiteCapability::Merkle => self.disable_merkle,
            LiteCapability::ToolSchema => self.disable_tool_schema,
            LiteCapability::Canvas => self.disable_canvas,
            LiteCapability::Carapace => self.disable_carapace,
            LiteCapability::Attachments => self.disable_attachments,
            // LOW
            LiteCapability::Bench => self.disable_bench,
            LiteCapability::Browser => self.disable_browser,
            LiteCapability::BrowserPwa => self.disable_browser_pwa,
            LiteCapability::Gitclaw => self.disable_gitclaw,
            LiteCapability::Zeptoclaw => self.disable_zeptoclaw,
            LiteCapability::ZhChannels => self.disable_zh_channels,
            LiteCapability::OnchainReputation => self.disable_onchain_reputation,
            LiteCapability::RatingImprove => self.disable_rating_improve,
            LiteCapability::SkillPack => self.disable_skill_pack,
            LiteCapability::I18n => self.disable_i18n,
        }
    }
}

/// Named capabilities that lite mode can gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteCapability {
    Speculative,
    SubagentReviewer,
    SyntheticExamples,
    Learning,
    Brief,
    SseStream,
    Deterministic,
    /// (#6) Autonomous curator — skill grading cycle.
    Curator,
    /// Agent federation — multi-agent parallel task decomposition.
    Federation,
    /// OpenMolt integration registry — provider tool awareness.
    OpenMolt,
    /// Wave execution engine — parallel tool wave scheduling.
    Wave,
    /// Signal/Matrix enterprise channels — external messaging alerts.
    Channels,
    /// Config hot-reload — zero-downtime configuration updates.
    HotReload,
    // HIGH — security and infrastructure
    /// Prompt injection detection — scans LLM inputs/outputs.
    Injection,
    /// Secret leak detection — scans tool responses for exfiltration.
    Leaks,
    /// Structured tracing — JSON logging + Prometheus metrics.
    Tracing,
    /// DockerIsolation Docker isolation — per-tool container execution.
    DockerIsolation,
    /// Sandbox enforcement — per-channel filesystem isolation.
    SandboxEnforcement,
    /// Auto-checkpoints — snapshot before file-modifying tools.
    Checkpoints,
    /// Zero-LLM rule engine — rule-based task routing.
    Rules,
    /// Routines engine — time-based routine triggers.
    Routines,
    /// Voice I/O — TTS/STT for brief delivery and input.
    Voice,
    /// Self-update — check and perform binary updates.
    SelfUpdate,
    // MEDIUM
    /// Embedding cache — cached embeddings for memory recall.
    EmbeddingCache,
    /// Reciprocal rank fusion — hybrid vector+fulltext search.
    Rrf,
    /// Archive — data portability (export/import bundles).
    Archive,
    /// Backup verification — integrity checks on backup files.
    Backup,
    /// Runtime skill — trigger-based dynamic skill invocation.
    RuntimeSkill,
    /// Event trigger — external event → session routing.
    Trigger,
    /// Observability — Langfuse LLM tracing.
    Observability,
    /// Merkle audit — tamper-evident audit logging.
    Merkle,
    /// Tool schema — manifest loading/validation.
    ToolSchema,
    /// Canvas — visual/text workspace blocks.
    Canvas,
    /// Carapace — signed plugin loading system.
    Carapace,
    /// Attachments — file attachment rendering in context.
    Attachments,
    // LOW
    /// Benchmark suite — performance benchmarking.
    Bench,
    /// Browser — headless browser tool for web tasks.
    Browser,
    /// Browser PWA — progressive web app agent mode.
    BrowserPwa,
    /// Gitclaw — git-native agent lifecycle management.
    Gitclaw,
    /// Zeptoclaw — lightweight tool inventory registry.
    Zeptoclaw,
    /// Chinese channels — WeChat/DingTalk/QQ messaging adapters.
    ZhChannels,
    /// Onchain reputation — blockchain-based reputation scoring.
    OnchainReputation,
    /// Rating improve — operator rating feedback processing.
    RatingImprove,
    /// Skill pack — skill packaging and distribution.
    SkillPack,
    /// I18n — multi-language brief/report generation.
    I18n,
}

// ---------------------------------------------------------------------------
// Model-override flag file (live model switching, gap #29)
// ---------------------------------------------------------------------------

/// Write a model override that the session runner picks up on the next cycle.
/// Format: `provider/model` (e.g. `anthropic/claude-3-5-sonnet-latest`).
pub fn set_model_override(data_dir: &Path, model: &str) -> anyhow::Result<()> {
    let model = model.trim();
    anyhow::ensure!(!model.is_empty(), "model override must not be blank");
    fs::write(data_dir.join("model_override"), model)
        .map_err(|e| anyhow::anyhow!("failed to write model_override flag: {e}"))
}

/// Read the current model override, if any.
pub fn get_model_override(data_dir: &Path) -> Option<String> {
    let path = data_dir.join("model_override");
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Remove the model override flag file.
pub fn clear_model_override(data_dir: &Path) -> anyhow::Result<()> {
    let path = data_dir.join("model_override");
    if path.exists() {
        fs::remove_file(&path)?;
    }
    Ok(())
}

impl LiteMode {
    /// Create a "fast" lite mode that disables all expensive capabilities.
    /// Used by `/fast` command and `--fast` flag.
    pub fn fast_all() -> Self {
        Self {
            enabled: true,
            context_ceiling_pct: 50.0,
            disable_speculative: true,
            disable_subagent_reviewers: true,
            disable_synthetic_examples: true,
            disable_learning: true,
            disable_brief: true,
            disable_sse: true,
            disable_deterministic: true,
            disable_curator: true,
            disable_federation: true,
            disable_openmolt: true,
            disable_wave: true,
            disable_channels: true,
            disable_hotreload: true,
            anatomy_refresh_hours: 24,
            // HIGH
            disable_injection: true,
            disable_leaks: true,
            disable_tracing: true,
            disable_docker_isolation: true,
            disable_sandbox_enforcement: true,
            disable_checkpoints: true,
            disable_rules: true,
            disable_routines: true,
            disable_voice: true,
            disable_self_update: true,
            // MEDIUM
            disable_embedding_cache: true,
            disable_rrf: true,
            disable_archive: true,
            disable_backup: true,
            disable_runtime_skill: true,
            disable_trigger: true,
            disable_observability: true,
            disable_merkle: true,
            disable_tool_schema: true,
            disable_canvas: true,
            disable_carapace: true,
            disable_attachments: true,
            // LOW
            disable_bench: true,
            disable_browser: true,
            disable_browser_pwa: true,
            disable_gitclaw: true,
            disable_zeptoclaw: true,
            disable_zh_channels: true,
            disable_onchain_reputation: true,
            disable_rating_improve: true,
            disable_skill_pack: true,
            disable_i18n: true,
        }
    }

    /// Check if fast mode is active via a flag file.
    pub fn is_fast_active(data_dir: &Path) -> bool {
        data_dir.join("fast_mode").exists()
    }

    /// Toggle fast mode on/off via a flag file. Returns the new state.
    pub fn toggle_fast(data_dir: &Path) -> anyhow::Result<bool> {
        let flag = data_dir.join("fast_mode");
        if flag.exists() {
            fs::remove_file(&flag)?;
            Ok(false)
        } else {
            fs::write(&flag, "fast")?;
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn defaults_when_file_missing() {
        let dir = tempdir().unwrap();
        let lite = LiteMode::from_file(&dir.path().join("praxis.toml")).unwrap();
        assert!(!lite.enabled);
        assert!(!lite.skip_capability(LiteCapability::Speculative));
    }

    #[test]
    fn enabled_via_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("praxis.toml");
        fs::write(&path, "[agent]\nlite = true\n").unwrap();
        let lite = LiteMode::from_file(&path).unwrap();
        assert!(lite.enabled);
        assert!(lite.skip_capability(LiteCapability::Learning));
        assert!(lite.skip_capability(LiteCapability::Speculative));
    }

    #[test]
    fn disabled_skips_nothing() {
        let lite = LiteMode::default();
        assert!(!lite.skip_capability(LiteCapability::Speculative));
    }
}
