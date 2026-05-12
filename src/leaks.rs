//! Credential leak detection — Scan tool outputs for secret exfiltration.
//!
//! Scans both requests AND responses for secret exfiltration.
//! Credentials are injected at host boundary — WASM module never sees raw values.

use anyhow::Result;

/// Scan output for credential leaks.
pub fn detect_leaks(text: &str) -> Result<Vec<LeakFinding>> {
    let mut findings = Vec::new();

    // Common credential patterns
    let patterns = [
        ("OpenAI key", r"sk-[a-zA-Z0-9]{48}"),
        ("Stripe key", r"sk_live_[a-zA-Z0-9]{24,}"),
        ("GitHub token", r"ghp_[a-zA-Z0-9]{36}"),
        ("AWS key", r"AKIA[0-9A-Z]{16}"),
        ("Slack token", r"xox[baprs]-?[a-zA-Z0-9-]+"),
        ("Discord token", r"[a-z0-9]{24}\.[a-z0-9]{6}\.[a-z0-9]{27}"),
        ("Google API", r"AIza[0-9A-Za-z_-]{35}"),
        ("JWT", r"eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+"),
    ];

    for (name, pattern) in patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            for m in re.find_iter(text) {
                findings.push(LeakFinding {
                    credential_type: name.to_string(),
                    matched: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
    }

    Ok(findings)
}

/// Leak finding result.
#[derive(Debug, Clone)]
pub struct LeakFinding {
    /// Type of credential detected.
    pub credential_type: String,
    /// The matched string (partial redaction recommended in logs).
    pub matched: String,
    /// Start position in text.
    pub start: usize,
    /// End position in text.
    pub end: usize,
}

/// Policy for handling leaks.
#[derive(Debug, Clone)]
pub enum LeakPolicy {
    Block,
    Redact,
    Log,
}

/// Remove detected credentials from text.
pub fn redact(text: &str, findings: &[LeakFinding]) -> String {
    let mut result = text.to_string();
    for finding in findings {
        let redacted = "[REDACTED]".repeat(finding.matched.len() / 8);
        result.replace_range(finding.start..finding.end, &redacted);
    }
    result
}
