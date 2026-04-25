use anyhow::Result;

use crate::usage::{
    PhaseTokenUsage, ProviderAttempt, ProviderTokenSummary, ProviderUsageSummary,
    SessionTokenUsage, TokenLedgerSummary, TokenSummaryAllTime,
};

pub trait ProviderUsageStore {
    fn record_provider_attempts(&self, session_id: i64, attempts: &[ProviderAttempt])
    -> Result<()>;
    fn latest_provider_usage(&self) -> Result<Option<ProviderUsageSummary>>;
    fn latest_token_summary(&self) -> Result<Option<TokenLedgerSummary>>;
    fn latest_phase_token_usage(&self, limit: usize) -> Result<Vec<PhaseTokenUsage>>;
    fn token_summary_all_time(&self) -> Result<TokenSummaryAllTime>;
    fn token_usage_by_session(&self, limit: usize) -> Result<Vec<SessionTokenUsage>>;
    fn token_usage_by_provider(&self) -> Result<Vec<ProviderTokenSummary>>;
}
