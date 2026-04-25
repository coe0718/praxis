use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::storage::{ApprovalStatus, NewApprovalRequest, StoredApprovalRequest};

use super::SqliteSessionStore;

pub(super) fn queue_approval(
    store: &SqliteSessionStore,
    request: &NewApprovalRequest,
) -> Result<StoredApprovalRequest> {
    let connection = store.connect()?;
    let now = Utc::now().to_rfc3339();
    let write_paths =
        serde_json::to_string(&request.write_paths).context("failed to serialize write paths")?;

    connection
        .execute(
            "
            INSERT INTO approval_requests(
                tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?7)
            ",
            params![
                request.tool_name,
                request.summary,
                request.requested_by,
                write_paths,
                request.payload_json,
                request.status.as_str(),
                now,
            ],
        )
        .context("failed to queue approval request")?;

    get_approval(store, connection.last_insert_rowid())?
        .context("approval request disappeared after insert")
}

pub(super) fn search_approvals(
    store: &SqliteSessionStore,
    q: Option<&str>,
    tool: Option<&str>,
    status: Option<ApprovalStatus>,
) -> Result<Vec<StoredApprovalRequest>> {
    let connection = store.connect()?;

    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(q_val) = q {
        // Escape LIKE metacharacters so user input is literal.
        let escaped = q_val.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let pattern = format!("%{}%", escaped);
        conditions.push(
            "(tool_name LIKE ? ESCAPE '\\' OR summary LIKE ? ESCAPE '\\' OR requested_by LIKE ? ESCAPE '\\')"
                .to_string(),
        );
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern.clone()));
        params.push(Box::new(pattern));
    }
    if let Some(tool_val) = tool {
        conditions.push("tool_name = ?".to_string());
        params.push(Box::new(tool_val.to_string()));
    }
    if let Some(status_val) = status {
        conditions.push("status = ?".to_string());
        params.push(Box::new(status_val.as_str().to_string()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
         FROM approval_requests{}
         ORDER BY id DESC
         LIMIT 500",
        where_clause
    );

    let mut stmt = connection.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let rows = stmt.query_map(&param_refs[..], row_to_request)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to search approval queue")
}

pub(super) fn list_approvals(
    store: &SqliteSessionStore,
    status: Option<ApprovalStatus>,
) -> Result<Vec<StoredApprovalRequest>> {
    let connection = store.connect()?;
    if let Some(value) = status {
        let mut statement = connection.prepare(
            "
            SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
            FROM approval_requests
            WHERE status = ?1
            ORDER BY id DESC
            LIMIT 500
            ",
        )?;
        let rows = statement.query_map(params![value.as_str()], row_to_request)?;
        return rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to load approval queue");
    }

    let mut statement = connection.prepare(
        "
        SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
        FROM approval_requests
        ORDER BY id DESC
        LIMIT 500
        ",
    )?;
    let rows = statement.query_map([], row_to_request)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load approval queue")
}

pub(super) fn get_approval(
    store: &SqliteSessionStore,
    id: i64,
) -> Result<Option<StoredApprovalRequest>> {
    let connection = store.connect()?;
    connection
        .query_row(
            "
            SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
            FROM approval_requests
            WHERE id = ?1
            ",
            params![id],
            row_to_request,
        )
        .optional()
        .context("failed to load approval request")
}

pub(super) fn set_approval_status(
    store: &SqliteSessionStore,
    id: i64,
    status: ApprovalStatus,
    note: Option<&str>,
) -> Result<Option<StoredApprovalRequest>> {
    let connection = store.connect()?;
    connection
        .execute(
            "
            UPDATE approval_requests
            SET status = ?2, status_note = ?3, updated_at = ?4
            WHERE id = ?1
            ",
            params![id, status.as_str(), note, Utc::now().to_rfc3339()],
        )
        .context("failed to update approval request")?;

    // Read back on the same connection to avoid stale data.
    connection
        .query_row(
            "
            SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
            FROM approval_requests
            WHERE id = ?1
            ",
            params![id],
            row_to_request,
        )
        .optional()
        .context("failed to load updated approval request")
}

pub(super) fn next_approved_request(
    store: &SqliteSessionStore,
) -> Result<Option<StoredApprovalRequest>> {
    let mut connection = store.connect()?;
    // Use an IMMEDIATE transaction so concurrent callers cannot read the same
    // approved row — the second caller blocks until this transaction commits,
    // at which point the row is already 'claiming' and no longer matches.
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .context("failed to begin approval claim transaction")?;
    let result = tx
        .query_row(
            "
            SELECT id, tool_name, summary, requested_by, write_paths, payload_json, status, status_note, created_at, updated_at
            FROM approval_requests
            WHERE status = 'approved'
            ORDER BY id ASC
            LIMIT 1
            ",
            [],
            row_to_request,
        )
        .optional()
        .context("failed to load next approved request")?;
    if let Some(ref req) = result {
        tx.execute(
            "UPDATE approval_requests SET status = 'claiming', updated_at = ?2 WHERE id = ?1",
            params![req.id, Utc::now().to_rfc3339()],
        )
        .context("failed to claim approval request")?;
    }
    tx.commit().context("failed to commit approval claim")?;
    Ok(result)
}

pub(super) fn mark_approval_consumed(store: &SqliteSessionStore, id: i64) -> Result<()> {
    let connection = store.connect()?;
    connection
        .execute(
            "
            UPDATE approval_requests
            SET status = 'executed', updated_at = ?2
            WHERE id = ?1
            ",
            params![id, Utc::now().to_rfc3339()],
        )
        .context("failed to mark approval request as executed")?;
    Ok(())
}

fn row_to_request(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredApprovalRequest> {
    let status = ApprovalStatus::parse(&row.get::<_, String>(6)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        )
    })?;

    let write_paths =
        serde_json::from_str::<Vec<String>>(&row.get::<_, String>(4)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    Ok(StoredApprovalRequest {
        id: row.get(0)?,
        tool_name: row.get(1)?,
        summary: row.get(2)?,
        requested_by: row.get(3)?,
        write_paths,
        payload_json: row.get(5)?,
        status,
        status_note: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
