//! Kanban — dispatcher/worker task board for Praxis.
//!
//! Provides structured tool-call surface for orchestrator and worker agents.
//! Tools are only registered when the `kanban` feature is enabled or when
//! `PRAXIS_KANBAN_TASK` env var is set (dispatcher-spawned worker).
//!
//! ## Schema
//!
//! - `tasks` — core task rows with status, assignee, priority, labels
//! - `task_events` — append-only event log (created, started, blocked, etc.)
//! - `task_comments` — durable threaded comments
//! - `task_runs` — worker run history (attempt records)

pub mod db;
pub mod tools;
pub mod dispatcher;
pub mod cli;

pub use db::KanbanStore;