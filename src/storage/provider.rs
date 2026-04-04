use anyhow::Result;

use crate::usage::{ProviderAttempt, ProviderUsageSummary};

pub trait ProviderUsageStore {
    fn record_provider_attempts(&self, session_id: i64, attempts: &[ProviderAttempt])
    -> Result<()>;
    fn latest_provider_usage(&self) -> Result<Option<ProviderUsageSummary>>;
}
