//! Kanban dispatcher — polls for ready tasks and spawns worker sessions.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};

use super::db::KanbanStore;
use crate::paths::PraxisPaths;

/// Global kanban store — initialized once at startup.
static KANBAN_STORE: Mutex<Option<Arc<KanbanStore>>> = Mutex::new(None);

/// Initialize the global kanban store. Call once at startup.
pub fn init_store(paths: &PraxisPaths) -> Result<()> {
    let store = KanbanStore::new(&paths.kanban_db_file)?;
    let mut guard = KANBAN_STORE.lock().unwrap();
    *guard = Some(Arc::new(store));
    Ok(())
}

/// Get a reference to the global kanban store.
pub fn get_store() -> Result<Arc<KanbanStore>> {
    let guard = KANBAN_STORE.lock().unwrap();
    guard.clone().context("kanban store not initialized — call init_store first")
}

/// Dispatcher state for tracking active workers.
pub struct Dispatcher {
    /// task_id → worker PID
    active_workers: HashMap<String, u32>,
    /// How long (seconds) before a worker with no heartbeat is considered stale.
    stale_threshold_secs: i64,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self {
            active_workers: HashMap::new(),
            stale_threshold_secs: 300, // 5 minutes
        }
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Run one dispatch tick — reclaim stale workers, spawn for ready tasks.
    pub fn tick(&mut self, paths: &PraxisPaths) -> Result<Vec<String>> {
        let store = get_store()?;
        let mut spawned = Vec::new();

        // 1. Reclaim stale workers
        for task_id in store.stale_workers(self.stale_threshold_secs)? {
            store.update_status(&task_id, super::db::TaskStatus::Ready)?;
            log::info!("kanban: reclaimed stale task {task_id}");
        }

        // 2. Spawn workers for ready tasks that have no active worker
        let ready_tasks = store.list_tasks(Some(super::db::TaskStatus::Ready), None, 10)?;
        for task in ready_tasks {
            if !self.active_workers.contains_key(&task.id) {
                let pid = self.spawn_worker(paths, &task.id)?;
                self.active_workers.insert(task.id.clone(), pid);
                spawned.push(task.id.clone());
                log::info!("kanban: spawned worker {} for task {}", pid, task.id);
            }
        }

        Ok(spawned)
    }

    /// Spawn a praxis worker session for the given task.
    /// Sets PRAXIS_KANBAN_TASK=<task_id> in the child environment.
    fn spawn_worker(&self, paths: &PraxisPaths, task_id: &str) -> Result<u32> {
        let executable = std::env::current_exe().context("failed to resolve current executable")?;
        let log_dir = paths.data_dir.join("kanban_logs");
        std::fs::create_dir_all(&log_dir)?;
        let log_file = log_dir.join(format!("{task_id}.log"));

        let mut child = std::process::Command::new(&executable);
        child
            .args(["run", "--task", task_id])
            .env("PRAXIS_KANBAN_TASK", task_id)
            .env("PRAXIS_DATA_DIR", paths.data_dir.to_string_lossy().as_ref())
            .stdout(std::fs::OpenOptions::new().create(true).append(true).open(&log_file)?)
            .stderr(std::fs::OpenOptions::new().create(true).append(true).open(&log_file)?);

        #[cfg(unix)]
        {}

        let child = child
            .spawn()
            .with_context(|| format!("failed to spawn kanban worker for task {task_id}"))?;
        Ok(child.id())
    }

    /// Mark a worker as done (removes from active pool).
    pub fn mark_done(&mut self, task_id: &str) {
        self.active_workers.remove(task_id);
    }

    /// Run the dispatch loop indefinitely.
    pub fn run_loop(&mut self, paths: &PraxisPaths) {
        loop {
            match self.tick(paths) {
                Ok(spawned) => {
                    if !spawned.is_empty() {
                        log::info!("kanban dispatcher: spawned {} workers", spawned.len());
                    }
                }
                Err(e) => {
                    log::error!("kanban dispatcher tick error: {e}");
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }
}

impl super::db::TaskStatus {
    // Alias for use in dispatcher
}
