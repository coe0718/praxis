//! `praxis kanban` CLI subcommands.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::kanban::db::{TaskPriority, TaskStatus};
use crate::kanban::dispatcher;
use crate::paths::PraxisPaths;

#[derive(Parser, Debug)]
pub struct KanbanCli {
    #[command(subcommand)]
    pub command: KanbanCommand,
}

#[derive(Subcommand, Debug)]
pub enum KanbanCommand {
    /// Create a new kanban task
    Create {
        title: String,
        #[arg(short, long)]
        body: Option<String>,
        #[arg(short, long, default_value = "medium")]
        priority: String,
        #[arg(short, long)]
        assignee: Option<String>,
    },
    /// List tasks
    List {
        #[arg(short, long)]
        status: Option<String>,
        #[arg(short, long)]
        assignee: Option<String>,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Show a task's full state
    Show { task_id: String },
    /// Complete a task
    Complete {
        task_id: String,
        #[arg(short, long)]
        summary: Option<String>,
    },
    /// Block a task
    Block {
        task_id: String,
        #[arg(short, long)]
        reason: String,
    },
    /// Unblock a task
    Unblock { task_id: String },
    /// Take/claim a task
    Take {
        task_id: String,
        #[arg(short, long)]
        assignee: Option<String>,
    },
    /// Link a task to a parent
    Link {
        task_id: String,
        #[arg(short, long)]
        parent_id: String,
    },
    /// Run one dispatcher tick
    Dispatch,
    /// Run the dispatcher loop continuously
    Serve,
}

impl KanbanCli {
    pub fn run(&self) -> Result<()> {
        let data_dir = crate::paths::default_data_dir().ok();
        let data_dir = data_dir
            .or_else(|| {
                Some(
                    crate::paths::default_data_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp/praxis")),
                )
            })
            .unwrap();
        let paths = PraxisPaths::for_data_dir(data_dir);
        dispatcher::init_store(&paths)?;
        let store = dispatcher::get_store()?;

        match &self.command {
            KanbanCommand::Create {
                title,
                body,
                priority,
                assignee,
            } => {
                let p = TaskPriority::parse_from_str(priority)
                    .context("invalid priority — use low|medium|high")?;
                let task = store.create_task(
                    title,
                    body.as_deref(),
                    p,
                    assignee.as_deref(),
                    vec![],
                    vec![],
                )?;
                println!("Created task {}: {}", task.id, task.title);
            }
            KanbanCommand::List { status, assignee, limit } => {
                let s = status.as_ref().and_then(|v| TaskStatus::parse_from_str(v));
                let tasks = store.list_tasks(s, assignee.as_deref(), *limit)?;
                let count = tasks.len();
                if tasks.is_empty() {
                    println!("No tasks found.");
                    return Ok(());
                }
                for task in tasks {
                    println!(
                        "[{:>10}] {:12} {:12} | {} | {}",
                        task.id,
                        task.status.as_str(),
                        task.priority.as_str(),
                        task.assignee.unwrap_or_default(),
                        task.title,
                    );
                }
                println!("\n{} task(s)", count);
            }
            KanbanCommand::Show { task_id } => {
                let task = store.get_task(task_id)?;
                let children = store.get_children(task_id)?;
                let comments = store.get_comments(task_id)?;
                let events = store.get_events(task_id, 20)?;
                let runs = store.get_runs(task_id)?;
                println!("=== Task {} ===", task_id);
                println!("Title:    {}", task.title);
                println!("Status:   {}", task.status.as_str());
                println!("Priority: {}", task.priority.as_str());
                println!("Assignee: {:?}", task.assignee);
                if let Some(r) = task.blocked_reason {
                    println!("Blocked:  {}", r);
                }
                if let Some(b) = task.body {
                    println!("Body:\n{}", b);
                }
                println!("Children: {} task(s)", children.len());
                println!("Comments: {}", comments.len());
                println!("Events:   {} recent", events.len());
                println!("Runs:     {} total", runs.len());
            }
            KanbanCommand::Complete { task_id, summary } => {
                let s = summary.as_deref().unwrap_or("Completed.");
                store.complete_task(task_id, s)?;
                println!("Task {} marked done.", task_id);
            }
            KanbanCommand::Block { task_id, reason } => {
                store.block_task(task_id, reason)?;
                println!("Task {} blocked: {}", task_id, reason);
            }
            KanbanCommand::Unblock { task_id } => {
                store.unblock_task(task_id)?;
                println!("Task {} unblocked.", task_id);
            }
            KanbanCommand::Take { task_id, assignee } => {
                let who = assignee.as_deref().unwrap_or("agent");
                let pid = std::process::id() as i64;
                store.take_task(task_id, who, pid)?;
                println!("Task {} claimed by {} (pid={})", task_id, who, pid);
            }
            KanbanCommand::Link { task_id, parent_id } => {
                store.link_task(task_id, parent_id)?;
                println!("Linked {} → parent {}", task_id, parent_id);
            }
            KanbanCommand::Dispatch => {
                let mut d = dispatcher::Dispatcher::new();
                let spawned = d.tick(&paths)?;
                if spawned.is_empty() {
                    println!("No new workers spawned this tick.");
                } else {
                    println!("Spawned {} worker(s): {:?}", spawned.len(), spawned);
                }
            }
            KanbanCommand::Serve => {
                println!("Starting kanban dispatcher loop...");
                let mut d = dispatcher::Dispatcher::new();
                d.run_loop(&paths);
            }
        }
        Ok(())
    }
}
