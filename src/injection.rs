//! Prompt injection detection — Detect and sanitize malicious inputs.
//!
//! IronClaw's innovation: Pattern detection + content sanitization.
//! Prevents LLM manipulation via crafted inputs.

use anyhow::Result;

/// Scan input for prompt injection patterns.
pub fn detect_injection(text: &str) -> Result<Vec<InjectionFinding>> {
    let mut findings = Vec::new();

    // Common injection patterns
    let patterns = [
        ("Jailbreak", r"(?i)(ignore.*[previous|all].*instruction|disregard.*system)"),
        ("Roleplay", r"(?i)(you are now|act as|pretend to be|roleplay as)"),
        ("Delimiter abuse", r"(?i)(<\|.*?\|>|<\?.*?\?>|\{\{.*?\}\})"),
        ("Instruction override", r"(?i)(new instruction|override|replace.*system)"),
        ("DAN mode", r"(?i)(do anything now|dan mode|unfiltered)"),
    ];

    for (pattern_name, pattern) in patterns {
        if regex::Regex::new(pattern)?.is_match(text) {
            findings.push(InjectionFinding {
                pattern: pattern_name.to_string(),
                severity: Severity::Medium,
            });
        }
    }

    Ok(findings)
}

/// Injection finding result.
#[derive(Debug, Clone)]
pub struct InjectionFinding {
    /// Name of the detected pattern.
    pub pattern: String,
    /// Severity level.
    pub severity: Severity,
}

/// Severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Sanitize input by removing injection patterns.
pub fn sanitize_input(text: &str) -> Result<String> {
    // Remove common injection markers
    let sanitized = text.replace("<|", "").replace("|>", "").replace("<?", "").replace("?>", "");

    Ok(sanitized)
}

/// Policy action for detected injections.
#[derive(Debug, Clone)]
pub enum PolicyAction {
    Allow,
    Warn,
    Block,
    Sanitize,
}
