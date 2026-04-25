//! Interactive setup wizard for `praxis init`.
//!
//! When `praxis init` is run with no CLI flags and no existing `praxis.toml`,
//! the wizard walks the operator through each config section with prompts,
//! defaults, and validation.  If any flag is provided (`--name`, `--timezone`,
//! `--security-level`), the wizard is skipped and the non-interactive path is
//! used instead.

use anyhow::Result;
use dialoguer::{Input, Password, Select, theme::ColorfulTheme};

/// Modes that `praxis init` supports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitMode {
    /// All defaults — equivalent to `praxis init` with no flags and no wizard.
    Quick,
    /// Interactive wizard with prompts for each config section.
    Wizard,
    /// CLI flags provided — skip wizard entirely.
    FlagDriven,
}

pub struct WizardConfig {
    pub name: String,
    pub timezone: String,
    pub backend: String,
    pub api_key: Option<String>,
    pub security_level: u8,
    pub mode: InitMode,
}

impl WizardConfig {
    /// Determine the init mode from CLI args.
    pub fn detect_mode(args: &super::InitArgs) -> InitMode {
        // If any explicit flag is set, skip the wizard.
        if args.name != "Praxis" || args.timezone != "UTC" || args.security_level != 2 {
            return InitMode::FlagDriven;
        }
        InitMode::Wizard
    }
}

/// Run the interactive wizard, returning a fully-built config.
/// Returns `None` if the operator cancelled during the wizard.
pub fn run_wizard() -> Result<Option<WizardConfig>> {
    let theme = ColorfulTheme::default();

    println!();
    println!("  ╔══════════════════════════════════════╗");
    println!("  ║       Praxis Setup Wizard            ║");
    println!("  ╚══════════════════════════════════════╝");
    println!();
    println!("  This will help you configure a new Praxis agent.");
    println!("  Press Ctrl-C at any time to cancel.");
    println!();

    // ── Name ────────────────────────────────────────────────────────────
    let name: String = Input::with_theme(&theme)
        .with_prompt("Agent name")
        .default("Praxis".to_string())
        .interact_text()?;
    println!();

    // ── Timezone ────────────────────────────────────────────────────────
    let timezone: String = Input::with_theme(&theme)
        .with_prompt("Timezone")
        .default("America/New_York".to_string())
        .validate_with(|s: &String| -> Result<()> {
            crate::time::parse_timezone(s)
                .map(|_| ())
                .map_err(|_| anyhow::anyhow!("invalid timezone: {s}"))
        })
        .interact_text()?;
    println!();

    // ── Backend ─────────────────────────────────────────────────────────
    let backends = &["stub (no LLM — for testing)", "openai", "claude", "ollama"];
    let backend_idx = Select::with_theme(&theme)
        .with_prompt("LLM backend")
        .items(backends)
        .default(2) // claude
        .interact()?;
    let backend = match backend_idx {
        0 => "stub",
        1 => "openai",
        2 => "claude",
        3 => "ollama",
        _ => "claude",
    };
    println!();

    // ── API Key ─────────────────────────────────────────────────────────
    let api_key: Option<String> = if backend == "stub" || backend == "ollama" {
        None
    } else {
        let env_name = match backend {
            "openai" => "OPENAI_API_KEY",
            "claude" => "ANTHROPIC_API_KEY",
            _ => "API_KEY",
        };
        let prompt = format!("{} (leave empty to skip — set later via env var)", env_name);
        let key: String = Password::with_theme(&theme)
            .with_prompt(&prompt)
            .allow_empty_password(true)
            .interact()?;
        println!();
        if key.is_empty() {
            println!("  ⚠ No key provided. Set {} before running the agent.", env_name);
            None
        } else {
            Some(key)
        }
    };

    // ── Security Level ──────────────────────────────────────────────────
    let levels = &[
        "1 — Relaxed: auto-approve common tools, minimal friction",
        "2 — Standard (recommended): approve risky tools, rehearsal for shell",
        "3 — Strict: approve everything, rehearsal required for all shell actions",
    ];
    let level_idx = Select::with_theme(&theme)
        .with_prompt("Security level")
        .items(levels)
        .default(1)
        .interact()?;
    let security_level = (level_idx + 1) as u8;
    println!();

    // ── Summary ─────────────────────────────────────────────────────────
    println!("  ── Summary ──────────────────────────────");
    println!("  Name:           {name}");
    println!("  Timezone:       {timezone}");
    println!("  Backend:        {backend}");
    println!("  Security level: {security_level}");
    if let Some(ref key) = api_key {
        let masked: String = key
            .chars()
            .take(6)
            .chain(std::iter::repeat('*'))
            .take(key.len().min(16))
            .collect();
        println!("  API key:        {masked}");
    } else if backend != "stub" && backend != "ollama" {
        println!("  API key:        (not set — configure env var before running)");
    }
    println!();

    let confirm: bool = dialoguer::Confirm::with_theme(&theme)
        .with_prompt("Proceed with these settings?")
        .default(true)
        .interact()?;

    if !confirm {
        println!("\n  Setup cancelled. Run `praxis init` again to retry.");
        return Ok(None);
    }

    Ok(Some(WizardConfig {
        name,
        timezone,
        backend: backend.to_string(),
        api_key,
        security_level,
        mode: InitMode::Wizard,
    }))
}

/// Export the wizard-chosen API key to an environment variable for use
/// by the current process.  This is a convenience — the key is NOT persisted
/// in `praxis.toml`.  The operator should add it to their shell profile.
pub fn export_api_key_hint(backend: &str, key: Option<&str>) -> String {
    let env_name = match backend {
        "openai" => "OPENAI_API_KEY",
        "claude" => "ANTHROPIC_API_KEY",
        _ => return String::new(),
    };
    if let Some(k) = key {
        if k.is_empty() {
            return format!("Add `export {env_name}=<your-key>` to your shell profile.");
        }
        format!(
            "{env_name} exported for this session. Add `export {env_name}=<your-key>` to ~/.bashrc for persistence."
        )
    } else {
        format!(
            "{env_name} not set. Add `export {env_name}=<your-key>` to your shell profile before running the agent."
        )
    }
}
