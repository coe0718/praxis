//! Cost tracker — track API costs per session, tool, and provider.
//!
//! Records token usage and estimated costs for LLM and API calls.
//! Provides session-level and aggregate cost reporting.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pricing for a model (per million tokens).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per million input tokens (USD).
    pub input_per_m: f64,
    /// Cost per million output tokens (USD).
    pub output_per_m: f64,
}

impl ModelPricing {
    /// Calculate cost for given token counts.
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        (input_tokens as f64 * self.input_per_m / 1_000_000.0)
            + (output_tokens as f64 * self.output_per_m / 1_000_000.0)
    }
}

/// Known model pricing (per million tokens, USD).
pub fn known_pricing() -> HashMap<String, ModelPricing> {
    let mut m = HashMap::new();
    // OpenAI
    m.insert(
        "gpt-4o".to_string(),
        ModelPricing {
            input_per_m: 2.50,
            output_per_m: 10.0,
        },
    );
    m.insert(
        "gpt-4o-mini".to_string(),
        ModelPricing {
            input_per_m: 0.15,
            output_per_m: 0.60,
        },
    );
    m.insert(
        "gpt-4.1".to_string(),
        ModelPricing {
            input_per_m: 2.0,
            output_per_m: 8.0,
        },
    );
    m.insert(
        "gpt-4.1-mini".to_string(),
        ModelPricing {
            input_per_m: 0.40,
            output_per_m: 1.60,
        },
    );
    m.insert(
        "gpt-4.1-nano".to_string(),
        ModelPricing {
            input_per_m: 0.10,
            output_per_m: 0.40,
        },
    );
    m.insert(
        "o3".to_string(),
        ModelPricing {
            input_per_m: 10.0,
            output_per_m: 40.0,
        },
    );
    m.insert(
        "o4-mini".to_string(),
        ModelPricing {
            input_per_m: 1.50,
            output_per_m: 6.0,
        },
    );
    // Anthropic
    m.insert(
        "claude-sonnet-4".to_string(),
        ModelPricing {
            input_per_m: 3.0,
            output_per_m: 15.0,
        },
    );
    m.insert(
        "claude-haiku-3.5".to_string(),
        ModelPricing {
            input_per_m: 0.80,
            output_per_m: 4.0,
        },
    );
    // Google
    m.insert(
        "gemini-2.5-pro".to_string(),
        ModelPricing {
            input_per_m: 1.25,
            output_per_m: 10.0,
        },
    );
    m.insert(
        "gemini-2.5-flash".to_string(),
        ModelPricing {
            input_per_m: 0.15,
            output_per_m: 0.60,
        },
    );
    // Embeddings
    m.insert(
        "text-embedding-3-small".to_string(),
        ModelPricing {
            input_per_m: 0.02,
            output_per_m: 0.0,
        },
    );
    m.insert(
        "text-embedding-3-large".to_string(),
        ModelPricing {
            input_per_m: 0.13,
            output_per_m: 0.0,
        },
    );
    // TTS/STT
    m.insert(
        "tts-1".to_string(),
        ModelPricing {
            input_per_m: 15.0,
            output_per_m: 0.0,
        },
    );
    m.insert(
        "whisper-1".to_string(),
        ModelPricing {
            input_per_m: 6.0,
            output_per_m: 0.0,
        },
    );
    m
}

/// A single cost entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    /// Timestamp of the API call.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Session ID.
    pub session_id: String,
    /// Model used.
    pub model: String,
    /// Provider (openai, anthropic, etc.).
    pub provider: String,
    /// Input token count.
    pub input_tokens: u64,
    /// Output token count.
    pub output_tokens: u64,
    /// Estimated cost in USD.
    pub cost_usd: f64,
    /// Tool that triggered this call.
    pub tool: Option<String>,
}

/// Cost tracker for recording and reporting API costs.
#[derive(Debug, Clone, Default)]
pub struct CostTracker {
    entries: Vec<CostEntry>,
    custom_pricing: HashMap<String, ModelPricing>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom pricing for a model.
    pub fn set_pricing(&mut self, model: &str, pricing: ModelPricing) {
        self.custom_pricing.insert(model.to_string(), pricing);
    }

    /// Record a cost entry.
    pub fn record(
        &mut self,
        session_id: &str,
        model: &str,
        provider: &str,
        input_tokens: u64,
        output_tokens: u64,
        tool: Option<&str>,
    ) -> f64 {
        let pricing = self.get_pricing(model);
        let cost_usd = pricing.calculate(input_tokens, output_tokens);

        self.entries.push(CostEntry {
            timestamp: chrono::Utc::now(),
            session_id: session_id.to_string(),
            model: model.to_string(),
            provider: provider.to_string(),
            input_tokens,
            output_tokens,
            cost_usd,
            tool: tool.map(|s| s.to_string()),
        });

        cost_usd
    }

    /// Get pricing for a model (custom > known > default).
    fn get_pricing(&self, model: &str) -> ModelPricing {
        if let Some(p) = self.custom_pricing.get(model) {
            return p.clone();
        }
        let known = known_pricing();
        known.get(model).cloned().unwrap_or(ModelPricing {
            input_per_m: 1.0,
            output_per_m: 3.0,
        })
    }

    /// Total cost across all entries.
    pub fn total_cost(&self) -> f64 {
        self.entries.iter().map(|e| e.cost_usd).sum()
    }

    /// Total cost for a specific session.
    pub fn session_cost(&self, session_id: &str) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.session_id == session_id)
            .map(|e| e.cost_usd)
            .sum()
    }

    /// Total cost for a specific tool.
    pub fn tool_cost(&self, tool: &str) -> f64 {
        self.entries
            .iter()
            .filter(|e| e.tool.as_deref() == Some(tool))
            .map(|e| e.cost_usd)
            .sum()
    }

    /// Cost breakdown by model.
    pub fn cost_by_model(&self) -> HashMap<String, f64> {
        let mut map: HashMap<String, f64> = HashMap::new();
        for e in &self.entries {
            *map.entry(e.model.clone()).or_default() += e.cost_usd;
        }
        map
    }

    /// Token usage summary.
    pub fn token_summary(&self) -> (u64, u64) {
        let input: u64 = self.entries.iter().map(|e| e.input_tokens).sum();
        let output: u64 = self.entries.iter().map(|e| e.output_tokens).sum();
        (input, output)
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether tracker is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    pub fn entries(&self) -> &[CostEntry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_pricing_calc() {
        let p = ModelPricing {
            input_per_m: 2.50,
            output_per_m: 10.0,
        };
        // 1000 input + 500 output tokens
        let cost = p.calculate(1000, 500);
        let expected = (1000.0 * 2.50 / 1_000_000.0) + (500.0 * 10.0 / 1_000_000.0);
        assert!((cost - expected).abs() < 0.0001);
    }

    #[test]
    fn test_record_and_total() {
        let mut tracker = CostTracker::new();
        tracker.record("s1", "gpt-4o", "openai", 1000, 500, Some("search"));
        tracker.record("s1", "gpt-4o", "openai", 2000, 1000, Some("code"));
        tracker.record("s2", "gpt-4o-mini", "openai", 500, 200, None);

        assert!(tracker.total_cost() > 0.0);
        assert!(tracker.session_cost("s1") > tracker.session_cost("s2"));
    }

    #[test]
    fn test_tool_cost() {
        let mut tracker = CostTracker::new();
        tracker.record("s1", "gpt-4o", "openai", 1000, 500, Some("search"));
        tracker.record("s1", "gpt-4o", "openai", 2000, 1000, Some("code"));

        assert!(tracker.tool_cost("search") > 0.0);
        assert!(tracker.tool_cost("code") > tracker.tool_cost("search"));
        assert_eq!(tracker.tool_cost("nonexistent"), 0.0);
    }

    #[test]
    fn test_cost_by_model() {
        let mut tracker = CostTracker::new();
        tracker.record("s1", "gpt-4o", "openai", 1000, 500, None);
        tracker.record("s1", "gpt-4o-mini", "openai", 1000, 500, None);

        let by_model = tracker.cost_by_model();
        assert_eq!(by_model.len(), 2);
        assert!(by_model.contains_key("gpt-4o"));
        assert!(by_model.contains_key("gpt-4o-mini"));
    }

    #[test]
    fn test_token_summary() {
        let mut tracker = CostTracker::new();
        tracker.record("s1", "gpt-4o", "openai", 1000, 500, None);
        tracker.record("s1", "gpt-4o", "openai", 2000, 1000, None);

        let (input, output) = tracker.token_summary();
        assert_eq!(input, 3000);
        assert_eq!(output, 1500);
    }

    #[test]
    fn test_custom_pricing() {
        let mut tracker = CostTracker::new();
        tracker.set_pricing(
            "custom-model",
            ModelPricing {
                input_per_m: 0.01,
                output_per_m: 0.01,
            },
        );
        tracker.record("s1", "custom-model", "custom", 1_000_000, 1_000_000, None);
        assert!((tracker.total_cost() - 0.02).abs() < 0.001);
    }
}
