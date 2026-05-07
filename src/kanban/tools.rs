//! Kanban tool implementations — dispatched from execute_request.

use anyhow::{Context, Result, bail};

use crate::paths::PraxisPaths;
use crate::storage::StoredApprovalRequest;
use crate::tools::parse_payload;

use super::db::{KanbanStore, TaskPriority, TaskStatus};
use super::dispatcher;

/// Returns the task ID this worker was dispatched with, or None if not a worker.
fn current_task_id() -> Option<String> {
    std::env::var("PRAXIS_KANBAN_TASK").ok()
}

/// Enforces worker ownership — rejects mutations on tasks the worker doesn't own.
/// This is the hallucination gate from Hermes commit #20232.
fn enforce_worker_ownership(_store: &KanbanStore, task_id: &str) -> Result<()> {
    if let Some(current) = current_task_id()
        && current != task_id {
            bail!(
                "Hallucination gate: task {task_id} is not assigned to this worker (owned={current}). \
                 Workers can only mutate their own task. This may indicate a prompt-injection attack."
            );
        }
    Ok(())
}

pub fn handle_kanban_create(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let title = params
        .get("title")
        .map(|v| v.as_str().to_string())
        .context("title is required")?;
    let body = params.get("body").map(|v| v.as_str().to_string());
    let priority_str = params
        .get("priority")
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| "medium".to_string());
    let assignee = params.get("assignee").map(|v| v.as_str().to_string());
    let parent_ids_str = params.get("parent_ids").map(|s| s.as_str()).unwrap_or("");
    let parent_ids: Vec<String> = if parent_ids_str.is_empty() {
        vec![]
    } else {
        parent_ids_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let labels_str = params.get("labels").map(|s| s.as_str()).unwrap_or("");
    let labels: Vec<String> = if labels_str.is_empty() {
        vec![]
    } else {
        labels_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    let priority =
        TaskPriority::from_str(&priority_str).context("invalid priority, use low|medium|high")?;

    let store = dispatcher::get_store()?;
    let task = store.create_task(
        &title,
        body.as_deref(),
        priority,
        assignee.as_deref(),
        parent_ids,
        labels,
    )?;

    Ok(serde_json::json!({ "ok": true, "task": task }).to_string())
}

pub fn handle_kanban_show(_paths: &PraxisPaths, request: &StoredApprovalRequest) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params
        .get("task_id")
        .map(|v| v.as_str().to_string())
        .or_else(|| std::env::var("PRAXIS_KANBAN_TASK").ok())
        .context("task_id is required (or set PRAXIS_KANBAN_TASK)")?;

    let store = dispatcher::get_store()?;
    let task = store.get_task(&task_id)?;
    let children = store.get_children(&task_id)?;
    let comments = store.get_comments(&task_id)?;
    let events = store.get_events(&task_id, 20)?;
    let runs = store.get_runs(&task_id)?;

    let worker_context = format!(
        "[Kanban task {}] status={} priority={} assignee={:?}\n\
         Title: {}\n\
         Blocked: {:?}\n\
         Children: {} task(s)\n\
         Comments: {}",
        task_id,
        task.status.as_str(),
        task.priority.as_str(),
        task.assignee,
        task.title,
        task.blocked_reason,
        children.len(),
        comments.len(),
    );

    Ok(serde_json::json!({
        "ok": true,
        "task": task,
        "children": children,
        "comments": comments,
        "events": events,
        "runs": runs,
        "worker_context": worker_context,
    })
    .to_string())
}

pub fn handle_kanban_complete(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params
        .get("task_id")
        .map(|v| v.as_str().to_string())
        .or_else(|| std::env::var("PRAXIS_KANBAN_TASK").ok())
        .context("task_id is required")?;

    let store = dispatcher::get_store()?;
    enforce_worker_ownership(&store, &task_id)?;

    let summary = params.get("summary").map(|v| v.as_str()).unwrap_or("Task completed.");
    let metadata = params
        .get("metadata")
        .map(|s| serde_json::Value::String(s.as_str().to_string()));

    store.complete_task(&task_id, summary)?;
    if let Some(run) = store.get_runs(&task_id)?.first() {
        store.complete_run(run.id, "completed", Some(summary), metadata)?;
    }

    Ok(serde_json::json!({ "ok": true, "task_id": task_id, "summary": summary }).to_string())
}

pub fn handle_kanban_block(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params
        .get("task_id")
        .map(|v| v.as_str().to_string())
        .or_else(current_task_id)
        .context("task_id is required")?;
    let reason = params
        .get("reason")
        .map(|v| v.as_str())
        .context("reason is required for blocking")?;

    let store = dispatcher::get_store()?;
    enforce_worker_ownership(&store, &task_id)?;
    store.block_task(&task_id, reason)?;

    Ok(serde_json::json!({ "ok": true, "task_id": task_id, "reason": reason }).to_string())
}

pub fn handle_kanban_unblock(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params.get("task_id").map(|v| v.as_str()).context("task_id is required")?;

    let store = dispatcher::get_store()?;
    store.unblock_task(task_id)?;

    Ok(serde_json::json!({ "ok": true, "task_id": task_id }).to_string())
}

pub fn handle_kanban_take(_paths: &PraxisPaths, request: &StoredApprovalRequest) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params.get("task_id").map(|v| v.as_str()).context("task_id is required")?;
    let assignee = params.get("assignee").map(|v| v.as_str()).unwrap_or("agent");
    let pid = std::process::id() as i64;

    let store = dispatcher::get_store()?;
    store.take_task(task_id, assignee, pid)?;

    Ok(serde_json::json!({
        "ok": true,
        "task_id": task_id,
        "assignee": assignee,
        "worker_pid": pid,
    })
    .to_string())
}

pub fn handle_kanban_heartbeat(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params
        .get("task_id")
        .map(|v| v.as_str().to_string())
        .or_else(current_task_id)
        .context("task_id is required")?;
    let progress = params.get("progress").map(|v| v.as_str());
    let pid = std::process::id() as i64;

    let store = dispatcher::get_store()?;
    enforce_worker_ownership(&store, &task_id)?;
    store.heartbeat(&task_id, pid, progress)?;

    Ok(serde_json::json!({ "ok": true, "task_id": task_id, "worker_pid": pid }).to_string())
}

pub fn handle_kanban_comment(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params
        .get("task_id")
        .map(|v| v.as_str().to_string())
        .or_else(current_task_id)
        .context("task_id is required")?;
    let body = params.get("body").map(|v| v.as_str()).context("body is required")?;
    let author = params.get("author").map(|v| v.as_str()).unwrap_or("agent");

    let store = dispatcher::get_store()?;
    let comment = store.add_comment(&task_id, body, author)?;

    Ok(serde_json::json!({ "ok": true, "comment": comment }).to_string())
}

pub fn handle_kanban_link(_paths: &PraxisPaths, request: &StoredApprovalRequest) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let task_id = params.get("task_id").map(|v| v.as_str()).context("task_id is required")?;
    let parent_id = params.get("parent_id").map(|v| v.as_str()).context("parent_id is required")?;

    let store = dispatcher::get_store()?;
    store.link_task(task_id, parent_id)?;

    Ok(serde_json::json!({ "ok": true, "task_id": task_id, "parent_id": parent_id }).to_string())
}

pub fn handle_kanban_tasks(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<String> {
    let payload = parse_payload(request.payload_json.as_deref())?;
    let params = payload.params;

    let status_str = params.get("status").map(|v| v.as_str());
    let status = status_str.and_then(TaskStatus::from_str);
    let assignee = params.get("assignee").map(|v| v.as_str());
    let limit = params.get("limit").map(|s| s.as_str().parse::<usize>().unwrap_or(50));

    let store = dispatcher::get_store()?;
    let tasks = store.list_tasks(status, assignee, limit.unwrap_or(50))?;

    Ok(serde_json::json!({ "ok": true, "count": tasks.len(), "tasks": tasks }).to_string())
}
