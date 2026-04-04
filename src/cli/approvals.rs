use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{
    cli::{ApprovalActionArgs, QueueArgs},
    storage::{ApprovalStatus, ApprovalStore, SessionStore, SqliteSessionStore},
    tools::{FileToolRegistry, sync_capabilities},
};

use super::core::load_initialized_config;

pub(crate) fn handle_queue(data_dir_override: Option<PathBuf>, args: QueueArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    let status = if args.all {
        None
    } else {
        Some(ApprovalStatus::Pending)
    };
    let approvals = store.list_approvals(status)?;
    if approvals.is_empty() {
        return Ok("queue: empty".to_string());
    }

    Ok(approvals
        .into_iter()
        .map(|item| {
            format!(
                "#{} {} {} paths={}",
                item.id,
                item.status.as_str(),
                item.tool_name,
                item.write_paths.join(",")
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

pub(crate) fn handle_approval_action(
    data_dir_override: Option<PathBuf>,
    args: ApprovalActionArgs,
    status: ApprovalStatus,
) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    let updated = store
        .set_approval_status(args.id, status, args.note.as_deref())?
        .with_context(|| format!("approval request {} not found", args.id))?;
    sync_capabilities(&FileToolRegistry, &store, &paths)?;

    Ok(format!(
        "approval: {}\nid: {}\ntool: {}",
        updated.status.as_str(),
        updated.id,
        updated.tool_name
    ))
}
