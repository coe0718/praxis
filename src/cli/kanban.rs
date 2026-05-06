//! `praxis kanban` CLI subcommand.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::kanban::db::{TaskPriority, TaskStatus};
use crate::kanban::dispatcher;
use crate::paths::PraxisPaths;

#[derive(Parser, Debug)]
pub struct KanbanArgs {
    #[command(subcommand)]
    pub command: KanbanCommand,
}

#[derive(Subcommand, Debug)]
pub enum KanbanCommand {
    Create {
        title: String,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(short, long, default_value = "medium")]
        priority: String,
        #[arg(short, long)]
        assignee: Option<String>,
    },
    List {
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        assignee: Option<String>,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    Show {
        task_id: String,
    },
    Complete {
        task_id: String,
        #[arg(short, long)]
        summary: Option<String>,
    },
    Block {
        task_id: String,
        #[arg(short, long)]
        reason: String,
    },
    Unblock {
        task_id: String,
    },
    Take {
        task_id: String,
        #[arg(short, long)]
        assignee: Option<String>,
    },
    Link {
        task_id: String,
        #[arg(short, long)]
        parent_id: String,
    },
    Dispatch,
    Serve,
}

pub fn handle_kanban(data_dir: Option<std::path::PathBuf>, args: KanbanArgs) -> Result<String> {
    let data_dir = data_dir
        .or_else(|| {
            Some(
                crate::paths::default_data_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/praxis")),
            )
        })
        .unwrap();
    let paths = PraxisPaths::for_data_dir(data_dir);
    dispatcher::init_store(&paths).context("failed to init kanban store")?;
    let store = dispatcher::get_store().context("failed to get kanban store")?;

    match args.command {
        KanbanCommand::Create {
            title,
            body,
            priority,
            assignee,
        } => {
            let p = TaskPriority::from_str(&priority)
                .context("invalid priority — use low|medium|high")?;
            let task = store.create_task(
                &title,
                body.as_deref(),
                p,
                assignee.as_deref(),
                vec![],
                vec![],
            )?;
            Ok(format!("Created task {}: {}", task.id, task.title))
        }
        KanbanCommand::List { status, assignee, limit } => {
            let s = status.as_ref().and_then(|v| TaskStatus::from_str(v));
            let tasks = store.list_tasks(s, assignee.as_deref(), limit)?;
            if tasks.is_empty() {
                return Ok("No tasks found.".to_string());
            }
            let mut out = String::new();
            for task in &tasks {
                out.push_str(&format!(
                    "[{:>10}] {:12} {:12} | {:15} | {}\n",
                    task.id,
                    task.status.as_str(),
                    task.priority.as_str(),
                    task.assignee.as_deref().unwrap_or("-"),
                    task.title,
                ));
            }
            out.push_str(&format!("\n{} task(s)", tasks.len()));
            Ok(out)
        }
        KanbanCommand::Show { task_id } => {
            let task = store.get_task(&task_id).context("task not found")?;
            let children = store.get_children(&task_id)?;
            let comments = store.get_comments(&task_id)?;
            let events = store.get_events(&task_id, 20)?;
            let runs = store.get_runs(&task_id)?;
            let mut out = format!("=== Task {} ===\n", task_id);
            out.push_str(&format!("Title:    {}\n", task.title));
            out.push_str(&format!("Status:   {}\n", task.status.as_str()));
            out.push_str(&format!("Priority: {}\n", task.priority.as_str()));
            out.push_str(&format!("Assignee: {:?}\n", task.assignee));
            if let Some(r) = &task.blocked_reason {
                out.push_str(&format!("Blocked:  {}\n", r));
            }
            if let Some(b) = &task.body {
                out.push_str(&format!("Body:\n{}\n", b));
            }
            out.push_str(&format!("Children: {} task(s)\n", children.len()));
            out.push_str(&format!("Comments: {}\n", comments.len()));
            out.push_str(&format!("Events:   {} recent\n", events.len()));
            out.push_str(&format!("Runs:     {} total\n", runs.len()));
            Ok(out)
        }
        KanbanCommand::Complete { task_id, summary } => {
            let s = summary.as_deref().unwrap_or("Completed.");
            store.complete_task(&task_id, s).context("failed to complete task")?;
            Ok(format!("Task {} marked done.", task_id))
        }
        KanbanCommand::Block { task_id, reason } => {
            store.block_task(&task_id, &reason).context("failed to block task")?;
            Ok(format!("Task {} blocked: {}", task_id, reason))
        }
        KanbanCommand::Unblock { task_id } => {
            store.unblock_task(&task_id).context("failed to unblock task")?;
            Ok(format!("Task {} unblocked.", task_id))
        }
        KanbanCommand::Take { task_id, assignee } => {
            let who = assignee.as_deref().unwrap_or("agent");
            let pid = std::process::id() as i64;
            store.take_task(&task_id, who, pid).context("failed to take task")?;
            Ok(format!("Task {} claimed by {} (pid={})", task_id, who, pid))
        }
        KanbanCommand::Link { task_id, parent_id } => {
            store.link_task(&task_id, &parent_id).context("failed to link task")?;
            Ok(format!("Linked {} → parent {}", task_id, parent_id))
        }
        KanbanCommand::Dispatch => {
            let mut d = dispatcher::Dispatcher::new();
            let spawned = d.tick(&paths).context("dispatch tick failed")?;
            if spawned.is_empty() {
                Ok("No new workers spawned this tick.".to_string())
            } else {
                Ok(format!("Spawned {} worker(s): {:?}", spawned.len(), spawned))
            }
        }
        KanbanCommand::Serve => {
            let mut d = dispatcher::Dispatcher::new();
            d.run_loop(&paths);
            unreachable!("run_loop never returns")
        }
    }
}
