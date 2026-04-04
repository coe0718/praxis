use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderAttempt {
    pub phase: String,
    pub provider: String,
    pub model: String,
    pub success: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub estimated_cost_micros: i64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderUsageSummary {
    pub session_id: i64,
    pub attempt_count: i64,
    pub failure_count: i64,
    pub last_provider: String,
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLedgerSummary {
    pub session_id: i64,
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseTokenUsage {
    pub phase: String,
    pub provider: String,
    pub tokens_used: i64,
    pub estimated_cost_micros: i64,
}

impl ProviderAttempt {
    pub fn tokens_used(&self) -> i64 {
        self.input_tokens + self.output_tokens
    }
}
