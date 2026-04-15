use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::params;

use crate::storage::{DecisionReceiptStore, NewDecisionReceipt, StoredDecisionReceipt};

use super::SqliteSessionStore;

impl DecisionReceiptStore for SqliteSessionStore {
    fn record_decision(&self, receipt: &NewDecisionReceipt) -> Result<()> {
        let conn = self.connect()?;
        let sources_json = serde_json::to_string(&receipt.context_sources)
            .context("failed to serialize context sources")?;
        conn.execute(
            "
            INSERT INTO decision_receipts
                (session_started_at, reason_code, goal_id, chosen_action,
                 context_sources_json, confidence, approval_required, recorded_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                receipt.session_started_at.to_rfc3339(),
                receipt.reason_code,
                receipt.goal_id,
                receipt.chosen_action,
                sources_json,
                receipt.confidence,
                i64::from(receipt.approval_required),
                Utc::now().to_rfc3339(),
            ],
        )
        .context("failed to insert decision receipt")?;
        Ok(())
    }

    fn recent_decisions(&self, limit: usize) -> Result<Vec<StoredDecisionReceipt>> {
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, session_started_at, reason_code, goal_id, chosen_action,
                       context_sources_json, confidence, approval_required, recorded_at
                FROM decision_receipts
                ORDER BY id DESC
                LIMIT ?1
                ",
            )
            .context("failed to prepare decision receipt query")?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let sources_json: String = row.get(5)?;
                let sources: Vec<String> =
                    serde_json::from_str(&sources_json).unwrap_or_default();
                Ok(StoredDecisionReceipt {
                    id: row.get(0)?,
                    session_started_at: row.get(1)?,
                    reason_code: row.get(2)?,
                    goal_id: row.get(3)?,
                    chosen_action: row.get(4)?,
                    context_sources: sources,
                    confidence: row.get(6)?,
                    approval_required: row.get::<_, i64>(7)? != 0,
                    recorded_at: row.get(8)?,
                })
            })
            .context("failed to query decision receipts")?;

        let mut receipts: Vec<StoredDecisionReceipt> = rows
            .collect::<std::result::Result<_, _>>()
            .context("failed to collect decision receipts")?;
        receipts.reverse();
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use crate::storage::{DecisionReceiptStore, NewDecisionReceipt, SessionStore, SqliteSessionStore};

    #[test]
    fn records_and_retrieves_decisions() {
        let dir = tempdir().unwrap();
        let store = SqliteSessionStore::new(dir.path().join("praxis.db"));
        store.initialize().unwrap();

        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 10, 0, 0).unwrap();
        store
            .record_decision(&NewDecisionReceipt {
                session_started_at: now,
                reason_code: "goal_selected".to_string(),
                goal_id: Some("G-001".to_string()),
                chosen_action: "work on memory search".to_string(),
                context_sources: vec!["identity".to_string(), "memory_hot".to_string()],
                confidence: 0.82,
                approval_required: false,
            })
            .unwrap();

        let decisions = store.recent_decisions(10).unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].reason_code, "goal_selected");
        assert_eq!(decisions[0].goal_id.as_deref(), Some("G-001"));
        assert!((decisions[0].confidence - 0.82).abs() < 1e-9);
        assert_eq!(decisions[0].context_sources, vec!["identity", "memory_hot"]);
        assert!(!decisions[0].approval_required);
    }

    #[test]
    fn recent_decisions_returns_oldest_first() {
        let dir = tempdir().unwrap();
        let store = SqliteSessionStore::new(dir.path().join("praxis.db"));
        store.initialize().unwrap();

        let now = chrono::Utc.with_ymd_and_hms(2026, 4, 14, 10, 0, 0).unwrap();
        for code in ["task_selected", "goal_selected", "stop_condition_met"] {
            store
                .record_decision(&NewDecisionReceipt {
                    session_started_at: now,
                    reason_code: code.to_string(),
                    goal_id: None,
                    chosen_action: format!("action for {code}"),
                    context_sources: vec![],
                    confidence: 0.9,
                    approval_required: false,
                })
                .unwrap();
        }

        let decisions = store.recent_decisions(3).unwrap();
        assert_eq!(decisions.len(), 3);
        assert_eq!(decisions[0].reason_code, "task_selected");
        assert_eq!(decisions[2].reason_code, "stop_condition_met");
    }
}
