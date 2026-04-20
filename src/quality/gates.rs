//! Quality gates — lightweight deterministic checks applied before outputs or
//! tool results become operator-visible.
//!
//! A gate inspects a piece of content and returns one of four decisions:
//!
//! - `Pass` — content is clean, continue.
//! - `Redact(String)` — replace the content with a scrubbed version.
//! - `Block(String)` — reject the content entirely with a reason.
//! - `RetryWithFeedback(String)` — content needs improvement; provide structured
//!   feedback for the generator to act on.
//!
//! Gates are designed to be cheap.  They should not call the LLM.  Complex
//! review belongs in the `Reviewer` or `EvaluateLoop`.

/// The decision a gate returns after inspecting content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// Content is acceptable.
    Pass,
    /// Replace the content with the provided scrubbed version.
    Redact(String),
    /// Block delivery entirely.  The string is the reason.
    Block(String),
    /// Content is deliverable but needs revision.  The string is feedback for
    /// the generator.
    RetryWithFeedback(String),
}

/// A single deterministic quality check.
pub trait QualityGate: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, content: &str) -> GateDecision;
}

/// Run content through a pipeline of gates in order.
/// The first non-Pass decision wins.  If all gates pass, returns `Pass`.
pub struct GatePipeline {
    gates: Vec<Box<dyn QualityGate>>,
}

impl GatePipeline {
    pub fn new(gates: Vec<Box<dyn QualityGate>>) -> Self {
        Self { gates }
    }

    pub fn empty() -> Self {
        Self { gates: Vec::new() }
    }

    /// Add a gate to the end of the pipeline.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, gate: impl QualityGate + 'static) -> Self {
        self.gates.push(Box::new(gate));
        self
    }

    pub fn run(&self, content: &str) -> GateDecision {
        for gate in &self.gates {
            match gate.check(content) {
                GateDecision::Pass => {}
                decision => return decision,
            }
        }
        GateDecision::Pass
    }
}

// ---------------------------------------------------------------------------
// Built-in gates
// ---------------------------------------------------------------------------

/// Scrub likely credential patterns from content before it is stored or
/// delivered.  Replaces matches with a `[REDACTED]` placeholder.
///
/// The patterns are heuristic: they catch common formats but are not a
/// substitute for proper secrets management at the source.
pub struct CredentialScrubGate;

impl QualityGate for CredentialScrubGate {
    fn name(&self) -> &'static str {
        "credential-scrub"
    }

    fn check(&self, content: &str) -> GateDecision {
        let scrubbed = scrub_credentials(content);
        if scrubbed != content {
            GateDecision::Redact(scrubbed)
        } else {
            GateDecision::Pass
        }
    }
}

/// Block delivery if the content exceeds a byte limit.
pub struct MaxLengthGate {
    pub max_bytes: usize,
}

impl QualityGate for MaxLengthGate {
    fn name(&self) -> &'static str {
        "max-length"
    }

    fn check(&self, content: &str) -> GateDecision {
        if content.len() > self.max_bytes {
            GateDecision::RetryWithFeedback(format!(
                "Response exceeds the {} byte limit ({} bytes). Shorten it.",
                self.max_bytes,
                content.len()
            ))
        } else {
            GateDecision::Pass
        }
    }
}

/// Block delivery if any of the listed substrings appear in the content.
pub struct ForbiddenPhraseGate {
    pub phrases: Vec<String>,
    pub reason: &'static str,
}

impl QualityGate for ForbiddenPhraseGate {
    fn name(&self) -> &'static str {
        "forbidden-phrase"
    }

    fn check(&self, content: &str) -> GateDecision {
        let lower = content.to_lowercase();
        for phrase in &self.phrases {
            if lower.contains(phrase.as_str()) {
                return GateDecision::Block(format!(
                    "Content contains forbidden phrase '{}': {}",
                    phrase, self.reason
                ));
            }
        }
        GateDecision::Pass
    }
}

/// Block delivery if the content appears to be empty or whitespace-only.
pub struct NonEmptyGate;

impl QualityGate for NonEmptyGate {
    fn name(&self) -> &'static str {
        "non-empty"
    }

    fn check(&self, content: &str) -> GateDecision {
        if content.trim().is_empty() {
            GateDecision::Block("Response is empty.".to_string())
        } else {
            GateDecision::Pass
        }
    }
}

// ---------------------------------------------------------------------------
// Standard pipeline factory
// ---------------------------------------------------------------------------

/// Build the default delivery gate pipeline — the set of gates applied to
/// every operator-visible output from the agent.
pub fn default_delivery_pipeline() -> GatePipeline {
    GatePipeline::empty()
        .add(NonEmptyGate)
        .add(CredentialScrubGate)
}

// ---------------------------------------------------------------------------
// Credential scrubbing
// ---------------------------------------------------------------------------

fn scrub_credentials(content: &str) -> String {
    // Chain multiple prefix-based redaction passes.  Longer prefixes first so
    // "sk-ant-" wins over "sk-".
    let s = replace_prefixed_tokens(content, "sk-ant-", 20);
    let s = replace_prefixed_tokens(&s, "sk-", 20);
    replace_prefixed_tokens(&s, "Bearer ", 10)
}

fn replace_prefixed_tokens(content: &str, prefix: &str, min_suffix: usize) -> String {
    if !content.contains(prefix) {
        return content.to_string();
    }

    let mut out = String::with_capacity(content.len());
    let mut remaining = content;

    while let Some(start) = remaining.find(prefix) {
        out.push_str(&remaining[..start]);
        let after_prefix = &remaining[start + prefix.len()..];
        let token_len = after_prefix
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | '/' | '='))
            .count();
        if token_len >= min_suffix {
            out.push_str("[REDACTED]");
            remaining = &remaining[start + prefix.len() + token_len..];
        } else {
            out.push_str(prefix);
            remaining = after_prefix;
        }
    }

    out.push_str(remaining);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_scrub_gate_redacts_sk_keys() {
        let gate = CredentialScrubGate;
        let content = "Use this key: sk-abcdefghijklmnopqrstuvwxyz1234567890";
        match gate.check(content) {
            GateDecision::Redact(s) => {
                assert!(s.contains("[REDACTED]"));
                assert!(!s.contains("abcdefghijklmnopqrstuvwxyz"));
            }
            other => panic!("expected Redact, got {other:?}"),
        }
    }

    #[test]
    fn credential_scrub_passes_clean_content() {
        let gate = CredentialScrubGate;
        assert_eq!(gate.check("Hello, operator!"), GateDecision::Pass);
    }

    #[test]
    fn max_length_gate_blocks_oversized_content() {
        let gate = MaxLengthGate { max_bytes: 10 };
        match gate.check("This is longer than 10 bytes.") {
            GateDecision::RetryWithFeedback(msg) => assert!(msg.contains("limit")),
            other => panic!("expected RetryWithFeedback, got {other:?}"),
        }
    }

    #[test]
    fn forbidden_phrase_gate_blocks_matching_content() {
        let gate = ForbiddenPhraseGate {
            phrases: vec!["never do this".to_string()],
            reason: "hard boundary",
        };
        match gate.check("You should never do this.") {
            GateDecision::Block(msg) => assert!(msg.contains("hard boundary")),
            other => panic!("expected Block, got {other:?}"),
        }
    }

    #[test]
    fn non_empty_gate_blocks_blank_response() {
        let gate = NonEmptyGate;
        assert_eq!(
            gate.check("   \n"),
            GateDecision::Block("Response is empty.".to_string())
        );
        assert_eq!(gate.check("Hello"), GateDecision::Pass);
    }

    #[test]
    fn pipeline_returns_first_non_pass() {
        let pipeline = GatePipeline::empty()
            .add(NonEmptyGate)
            .add(MaxLengthGate { max_bytes: 5 });

        assert_eq!(
            pipeline.run(""),
            GateDecision::Block("Response is empty.".to_string())
        );
        match pipeline.run("Too long for the limit.") {
            GateDecision::RetryWithFeedback(_) => {}
            other => panic!("expected RetryWithFeedback, got {other:?}"),
        }
        assert_eq!(pipeline.run("Hi!"), GateDecision::Pass);
    }
}
