use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};

use crate::{
    storage::ProviderUsageStore,
    usage::{
        PhaseTokenUsage, ProviderAttempt, ProviderTokenSummary, ProviderUsageSummary,
        SessionTokenUsage, TokenLedgerSummary, TokenSummaryAllTime,
    },
};

use super::SqliteSessionStore;

pub(super) fn record_attempts(
    store: &SqliteSessionStore,
    session_id: i64,
    attempts: &[ProviderAttempt],
) -> Result<()> {
    let mut connection = store.connect()?;
    let tx = connection
        .transaction()
        .context("failed to begin provider recording transaction")?;
    for attempt in attempts {
        tx.execute(
            "
                INSERT INTO provider_attempts(
                    session_id, phase, provider, model, success,
                    input_tokens, output_tokens, estimated_cost_micros, error
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ",
            params![
                session_id,
                attempt.phase,
                attempt.provider,
                attempt.model,
                attempt.success,
                attempt.input_tokens,
                attempt.output_tokens,
                attempt.estimated_cost_micros,
                attempt.error,
            ],
        )
        .context("failed to insert provider attempt")?;
        tx.execute(
            "
                INSERT INTO token_ledger(
                    session_id, phase, provider, model,
                    input_tokens, output_tokens, estimated_cost_micros
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ",
            params![
                session_id,
                attempt.phase,
                attempt.provider,
                attempt.model,
                attempt.input_tokens,
                attempt.output_tokens,
                attempt.estimated_cost_micros,
            ],
        )
        .context("failed to insert token ledger row")?;
    }
    tx.commit()
        .context("failed to commit provider recording transaction")?;
    Ok(())
}

pub(super) fn latest_usage(store: &SqliteSessionStore) -> Result<Option<ProviderUsageSummary>> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT session_id,
                   COUNT(*) AS attempts,
                   SUM(CASE WHEN success THEN 0 ELSE 1 END) AS failures,
                   MAX(id) AS latest_attempt_id,
                   SUM(input_tokens + output_tokens) AS tokens_used,
                   SUM(estimated_cost_micros) AS estimated_cost_micros
            FROM provider_attempts
            WHERE session_id = (SELECT session_id FROM provider_attempts ORDER BY id DESC LIMIT 1)
            GROUP BY session_id
            ",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .optional()
        .context("failed to query latest provider usage")?
        .map_or(Ok(None), |row| {
            let provider = connection
                .query_row(
                    "SELECT provider FROM provider_attempts WHERE id = ?1",
                    params![row.3],
                    |provider_row| provider_row.get(0),
                )
                .context("failed to query latest provider name")?;
            Ok(Some(ProviderUsageSummary {
                session_id: row.0,
                attempt_count: row.1,
                failure_count: row.2,
                last_provider: provider,
                tokens_used: row.4,
                estimated_cost_micros: row.5,
            }))
        })
}

pub(super) fn latest_token_summary(
    store: &SqliteSessionStore,
) -> Result<Option<TokenLedgerSummary>> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT session_id,
                   SUM(input_tokens + output_tokens) AS tokens_used,
                   SUM(estimated_cost_micros) AS estimated_cost_micros
            FROM token_ledger
            WHERE session_id = (SELECT session_id FROM token_ledger ORDER BY id DESC LIMIT 1)
            GROUP BY session_id
            ",
            [],
            |row| {
                Ok(TokenLedgerSummary {
                    session_id: row.get(0)?,
                    tokens_used: row.get(1)?,
                    estimated_cost_micros: row.get(2)?,
                })
            },
        )
        .optional()
        .context("failed to query latest token summary")
}

pub(super) fn latest_phase_usage(
    store: &SqliteSessionStore,
    limit: usize,
) -> Result<Vec<PhaseTokenUsage>> {
    let connection = store.connect()?;
    let mut statement = connection
        .prepare(
            "
            SELECT phase,
                   provider,
                   SUM(input_tokens + output_tokens) AS tokens_used,
                   SUM(estimated_cost_micros) AS estimated_cost_micros
            FROM token_ledger
            WHERE session_id = (SELECT session_id FROM token_ledger ORDER BY id DESC LIMIT 1)
            GROUP BY phase, provider
            ORDER BY tokens_used DESC, estimated_cost_micros DESC
            LIMIT ?1
            ",
        )
        .context("failed to prepare token hotspot query")?;
    let rows = statement
        .query_map(params![limit as i64], |row| {
            Ok(PhaseTokenUsage {
                phase: row.get(0)?,
                provider: row.get(1)?,
                tokens_used: row.get(2)?,
                estimated_cost_micros: row.get(3)?,
            })
        })
        .context("failed to execute token hotspot query")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load token hotspots")
}

pub(super) fn token_summary_all_time(store: &SqliteSessionStore) -> Result<TokenSummaryAllTime> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT
                COALESCE(SUM(input_tokens + output_tokens), 0) AS total_tokens,
                COALESCE(SUM(estimated_cost_micros), 0) AS total_cost_micros,
                COUNT(DISTINCT session_id) AS total_sessions
            FROM token_ledger
            ",
            [],
            |row| {
                Ok(TokenSummaryAllTime {
                    total_tokens: row.get(0)?,
                    total_cost_micros: row.get(1)?,
                    total_sessions: row.get(2)?,
                })
            },
        )
        .context("failed to query all-time token summary")
}

pub(super) fn token_usage_by_session(
    store: &SqliteSessionStore,
    limit: usize,
) -> Result<Vec<SessionTokenUsage>> {
    let connection = store.connect()?;
    let mut statement = connection
        .prepare(
            "
            SELECT
                s.id AS session_id,
                s.day,
                COALESCE(SUM(t.input_tokens + t.output_tokens), 0) AS tokens_used,
                COALESCE(SUM(t.estimated_cost_micros), 0) AS estimated_cost_micros
            FROM sessions s
            LEFT JOIN token_ledger t ON s.id = t.session_id
            GROUP BY s.id, s.day
            ORDER BY s.id DESC
            LIMIT ?1
            ",
        )
        .context("failed to prepare session token query")?;
    let rows = statement
        .query_map(params![limit as i64], |row| {
            Ok(SessionTokenUsage {
                session_id: row.get(0)?,
                day: row.get(1)?,
                tokens_used: row.get(2)?,
                estimated_cost_micros: row.get(3)?,
            })
        })
        .context("failed to execute session token query")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load session token usage")
}

pub(super) fn token_usage_by_provider(
    store: &SqliteSessionStore,
) -> Result<Vec<ProviderTokenSummary>> {
    let connection = store.connect()?;
    let mut statement = connection
        .prepare(
            "
            SELECT
                provider,
                SUM(input_tokens + output_tokens) AS tokens_used,
                SUM(estimated_cost_micros) AS estimated_cost_micros
            FROM token_ledger
            GROUP BY provider
            ORDER BY tokens_used DESC
            ",
        )
        .context("failed to prepare provider token query")?;
    let rows = statement
        .query_map([], |row| {
            Ok(ProviderTokenSummary {
                provider: row.get(0)?,
                tokens_used: row.get(1)?,
                estimated_cost_micros: row.get(2)?,
            })
        })
        .context("failed to execute provider token query")?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to load provider token usage")
}

impl ProviderUsageStore for SqliteSessionStore {
    fn record_provider_attempts(
        &self,
        session_id: i64,
        attempts: &[ProviderAttempt],
    ) -> Result<()> {
        record_attempts(self, session_id, attempts)
    }

    fn latest_provider_usage(&self) -> Result<Option<ProviderUsageSummary>> {
        latest_usage(self)
    }

    fn latest_token_summary(&self) -> Result<Option<TokenLedgerSummary>> {
        latest_token_summary(self)
    }

    fn latest_phase_token_usage(&self, limit: usize) -> Result<Vec<PhaseTokenUsage>> {
        latest_phase_usage(self, limit)
    }

    fn token_summary_all_time(&self) -> Result<TokenSummaryAllTime> {
        token_summary_all_time(self)
    }

    fn token_usage_by_session(&self, limit: usize) -> Result<Vec<SessionTokenUsage>> {
        token_usage_by_session(self, limit)
    }

    fn token_usage_by_provider(&self) -> Result<Vec<ProviderTokenSummary>> {
        token_usage_by_provider(self)
    }
}
