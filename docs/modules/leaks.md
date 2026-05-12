# Credential Leak Detection

> Scan tool outputs for secret exfiltration — detects credentials in both requests and responses before they leave the system.

## Overview

The credential leak detection module scans text for common credential patterns using regular expressions. It supports detection of API keys, tokens, and secrets from OpenAI, Stripe, GitHub, AWS, Slack, Discord, Google, and JWT tokens. A `LeakPolicy` enum controls the response to detected leaks: `Block` (prevent the output), `Redact` (replace matched text with `[REDACTED]`), or `Log` (record but allow).

The `redact()` function replaces detected credential text with `[REDACTED]` repetition to maintain approximate visual structure while removing sensitive content.

**Current status:** Fully implemented. Used as a safety layer in tool execution and response processing.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `LeakFinding` | A single detection: credential type, matched string, start/end byte positions. |
| `LeakPolicy` | Enum: `Block`, `Redact`, `Log` — determines response to leak. |

### Detection Patterns

| Credential Type | Regex Pattern |
|----------------|---------------|
| OpenAI key | `sk-[a-zA-Z0-9]{48}` |
| Stripe key | `sk_live_[a-zA-Z0-9]{24,}` |
| GitHub token | `ghp_[a-zA-Z0-9]{36}` |
| AWS key | `AKIA[0-9A-Z]{16}` |
| Slack token | `xox[baprs]-?[a-zA-Z0-9-]+` |
| Discord token | `[a-z0-9]{24}\.[a-z0-9]{6}\.[a-z0-9]{27}` |
| Google API | `AIza[0-9A-Za-z_-]{35}` |
| JWT | `eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+` |

## Public API

### Detection

```rust
pub fn detect_leaks(text: &str) -> Result<Vec<LeakFinding>>
```

Scans text against all credential patterns. Returns a list of findings (may be empty). Each finding includes the credential type, the matched string, and byte offsets.

### `LeakFinding`

```rust
pub struct LeakFinding {
    pub credential_type: String,
    pub matched: String,
    pub start: usize,
    pub end: usize,
}
```

### `LeakPolicy`

```rust
pub enum LeakPolicy {
    Block,
    Redact,
    Log,
}
```

### Redaction

```rust
pub fn redact(text: &str, findings: &[LeakFinding]) -> String
```

Replaces each matched span with `[REDACTED]` repeated `matched.len() / 8` times.

## Configuration

No `praxis.toml` fields. Used programmatically as a safety layer:

```rust
use praxis::leaks::{detect_leaks, redact, LeakPolicy};

fn process_tool_output(output: &str) -> Result<String, String> {
    let findings = detect_leaks(output).unwrap();

    if !findings.is_empty() {
        match LeakPolicy::Redact {
            LeakPolicy::Block => {
                return Err("Credential leak detected, output blocked".into());
            }
            LeakPolicy::Redact => {
                let safe = redact(output, &findings);
                return Ok(safe);
            }
            LeakPolicy::Log => {
                log::warn!("Credential leak detected in output: {:?}", &findings);
                // Fall through to use original output
            }
        }
    }

    Ok(output.to_string())
}
```

## Dependencies

- **`regex`** — Rust regex crate for pattern matching (8 patterns).
- **`anyhow`** — Error handling.

## Status

- ✅ 8 credential patterns (OpenAI, Stripe, GitHub, AWS, Slack, Discord, Google, JWT)
- ✅ Regex-based detection with byte offset positions
- ✅ Three response policies (Block, Redact, Log)
- ✅ Redaction function with visual structure preservation
- ⏳ Additional credential patterns (database URLs, SSH keys, etc.)
- ⏳ Integration with tool output pipeline

## Source

`src/leaks.rs`