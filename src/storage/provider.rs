use anyhow::Result;

use crate::usage::{PhaseTokenUsage, ProviderAttempt, ProviderUsageSummary, TokenLedgerSummary};

pub trait ProviderUsageStore {
    fn record_provider_attempts(&self, session_id: i64, attempts: &[ProviderAttempt])
    -> Result<()>;
    fn latest_provider_usage(&self) -> Result<Option<ProviderUsageSummary>>;
    fn latest_token_summary(&self) -> Result<Option<TokenLedgerSummary>>;
    fn latest_phase_token_usage(&self, limit: usize) -> Result<Vec<PhaseTokenUsage>>;
}
