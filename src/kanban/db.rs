//! Kanban SQLite database layer.

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Backlog,
    Ready,
    InProgress,
    Blocked,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "backlog" => Some(Self::Backlog),
            "ready" => Some(Self::Ready),
            "in_progress" => Some(Self::InProgress),
            "blocked" => Some(Self::Blocked),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl TaskPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee: Option<String>,
    pub parent_ids: Vec<String>,
    pub labels: Vec<String>,
    pub blocked_reason: Option<String>,
    pub worker_pid: Option<i64>,
    pub heartbeat_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: i64,
    pub task_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub actor: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskComment {
    pub id: i64,
    pub task_id: String,
    pub body: String,
    pub author: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRun {
    pub id: i64,
    pub task_id: String,
    pub session_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub summary: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub struct KanbanStore {
    conn: Mutex<Connection>,
}

impl KanbanStore {
    pub fn new(db_path: &PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("failed to open kanban db at {}", db_path.display()))?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                body        TEXT,
                status      TEXT NOT NULL DEFAULT 'backlog',
                priority    TEXT NOT NULL DEFAULT 'medium',
                assignee    TEXT,
                parent_ids  TEXT NOT NULL DEFAULT '[]',
                labels      TEXT NOT NULL DEFAULT '[]',
                blocked_reason TEXT,
                worker_pid  INTEGER,
                heartbeat_at TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE TABLE IF NOT EXISTS task_events (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id     TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                payload     TEXT NOT NULL DEFAULT '{}',
                actor       TEXT NOT NULL DEFAULT 'system',
                created_at  TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            CREATE TABLE IF NOT EXISTS task_comments (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id     TEXT NOT NULL,
                body        TEXT NOT NULL,
                author      TEXT NOT NULL DEFAULT 'agent',
                created_at  TEXT NOT NULL,
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            CREATE TABLE IF NOT EXISTS task_runs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id     TEXT NOT NULL,
                session_id  TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'started',
                started_at  TEXT NOT NULL,
                completed_at TEXT,
                summary     TEXT,
                metadata    TEXT,
                FOREIGN KEY (task_id) REFERENCES tasks(id)
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            CREATE INDEX IF NOT EXISTS idx_tasks_assignee ON tasks(assignee);
            CREATE INDEX IF NOT EXISTS idx_events_task ON task_events(task_id);
            CREATE INDEX IF NOT EXISTS idx_comments_task ON task_comments(task_id);
            CREATE INDEX IF NOT EXISTS idx_runs_task ON task_runs(task_id);
            "#,
        )
        .context("failed to initialize kanban schema")?;
        Ok(())
    }

    fn now() -> String {
        chrono_now()
    }

    fn parse_row_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let parent_ids_raw: String = row.get("parent_ids")?;
        let labels_raw: String = row.get("labels")?;
        Ok(Task {
            id: row.get("id")?,
            title: row.get("title")?,
            body: row.get("body")?,
            status: TaskStatus::from_str(&row.get::<_, String>("status")?).unwrap_or(TaskStatus::Backlog),
            priority: TaskPriority::from_str(&row.get::<_, String>("priority")?).unwrap_or(TaskPriority::Medium),
            assignee: row.get("assignee")?,
            parent_ids: serde_json::from_str(&parent_ids_raw).unwrap_or_default(),
            labels: serde_json::from_str(&labels_raw).unwrap_or_default(),
            blocked_reason: row.get("blocked_reason")?,
            worker_pid: row.get("worker_pid")?,
            heartbeat_at: row.get("heartbeat_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            completed_at: row.get("completed_at")?,
        })
    }

    // ── Task CRUD ──────────────────────────────────────────────────────────────

    pub fn create_task(
        &self,
        title: &str,
        body: Option<&str>,
        priority: TaskPriority,
        assignee: Option<&str>,
        parent_ids: Vec<String>,
        labels: Vec<String>,
    ) -> Result<Task> {
        let id = uuid_v4();
        let now = Self::now();
        let parent_json = serde_json::to_string(&parent_ids)?;
        let labels_json = serde_json::to_string(&labels)?;

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tasks (id, title, body, status, priority, assignee, parent_ids, labels, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'backlog', ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, title, body, priority.as_str(), assignee, parent_json, labels_json, now, now],
        )?;
        drop(conn);

        self.add_event(&id, "created", serde_json::json!({}), "system")?;
        self.get_task(&id)
    }

    pub fn get_task(&self, id: &str) -> Result<Task> {
        let conn = self.conn.lock().unwrap();
        let task = conn.query_row(
            "SELECT * FROM tasks WHERE id = ?1",
            params![id],
            Self::parse_row_task,
        )
        .context("task not found")?;
        Ok(task)
    }

    pub fn list_tasks(
        &self,
        status: Option<TaskStatus>,
        assignee: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut sql = String::from("SELECT * FROM tasks WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(s) = &status {
            sql.push_str(&format!(" AND status = '{}'", s.as_str()));
        }
        if let Some(a) = assignee {
            sql.push_str(" AND assignee = ?");
            params_vec.push(Box::new(a.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC");
        sql.push_str(&format!(" LIMIT {}", limit));

        let mut stmt = conn.prepare(&sql)?;
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let tasks = stmt
            .query_map(params_refs.as_slice(), Self::parse_row_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    pub fn update_status(&self, id: &str, status: TaskStatus) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now, id],
        )?;
        Ok(())
    }

    pub fn take_task(&self, id: &str, assignee: &str, worker_pid: i64) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'in_progress', assignee = ?1, worker_pid = ?2, heartbeat_at = ?3, updated_at = ?3 WHERE id = ?4",
            params![assignee, worker_pid, now, id],
        )?;
        drop(conn);
        self.add_event(id, "started", serde_json::json!({"assignee": assignee}), "system")
    }

    pub fn block_task(&self, id: &str, reason: &str) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'blocked', blocked_reason = ?1, updated_at = ?2 WHERE id = ?3",
            params![reason, now, id],
        )?;
        drop(conn);
        self.add_event(id, "blocked", serde_json::json!({"reason": reason}), "system")
    }

    pub fn unblock_task(&self, id: &str) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'ready', blocked_reason = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        drop(conn);
        self.add_event(id, "unblocked", serde_json::json!({}), "system")
    }

    pub fn complete_task(&self, id: &str, summary: &str) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE tasks SET status = 'done', completed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        drop(conn);
        self.add_event(id, "completed", serde_json::json!({"summary": summary}), "system")
    }

    pub fn heartbeat(&self, id: &str, pid: i64, progress: Option<&str>) -> Result<()> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        let mut updates = "worker_pid = ?1, heartbeat_at = ?2, updated_at = ?2".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(pid), Box::new(now)];

        if let Some(p) = progress {
            updates.push_str(", blocked_reason = ?3");
            params_vec.push(Box::new(p.to_string()));
        }
        updates.push_str(" WHERE id = ?3");
        params_vec.push(Box::new(id.to_string()));

        let sql = format!("UPDATE tasks SET {} ", updates);
        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())?;
        drop(conn);
        self.add_event(id, "heartbeat", serde_json::json!({"progress": progress}), "system")
    }

    pub fn get_children(&self, parent_id: &str) -> Result<Vec<Task>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM tasks WHERE parent_ids LIKE ?1 ORDER BY created_at DESC"
        )?;
        let pattern = format!("%\"{}\"%", parent_id);
        let tasks = stmt
            .query_map(params![pattern], Self::parse_row_task)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tasks)
    }

    pub fn stale_workers(&self, max_age_secs: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id FROM tasks
             WHERE status = 'in_progress'
               AND heartbeat_at IS NOT NULL
               AND heartbeat_at < datetime('now', ?1)"
        )?;
        let offset = format!("-{} seconds", max_age_secs);
        let ids = stmt
            .query_map(params![offset], |row| row.get("id"))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(ids)
    }

    // ── Events ─────────────────────────────────────────────────────────────────

    pub fn add_event(&self, task_id: &str, event_type: &str, payload: serde_json::Value, actor: &str) -> Result<()> {
        let now = Self::now();
        let payload_str = serde_json::to_string(&payload)?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO task_events (task_id, event_type, payload, actor, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task_id, event_type, payload_str, actor, now],
        )?;
        Ok(())
    }

    pub fn get_events(&self, task_id: &str, limit: usize) -> Result<Vec<TaskEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM task_events WHERE task_id = ?1 ORDER BY created_at DESC LIMIT ?2"
        )?;
        let events = stmt
            .query_map(params![task_id, limit], |row| {
                let payload_raw: String = row.get("payload")?;
                Ok(TaskEvent {
                    id: row.get("id")?,
                    task_id: row.get("task_id")?,
                    event_type: row.get("event_type")?,
                    payload: serde_json::from_str(&payload_raw).unwrap_or(serde_json::Value::Null),
                    actor: row.get("actor")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(events)
    }

    // ── Comments ───────────────────────────────────────────────────────────────

    pub fn add_comment(&self, task_id: &str, body: &str, author: &str) -> Result<TaskComment> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO task_comments (task_id, body, author, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![task_id, body, author, now],
        )?;
        let id = conn.last_insert_rowid();
        drop(conn);
        self.add_event(task_id, "commented", serde_json::json!({"body": body}), author)?;
        Ok(TaskComment {
            id,
            task_id: task_id.to_string(),
            body: body.to_string(),
            author: author.to_string(),
            created_at: now,
        })
    }

    pub fn get_comments(&self, task_id: &str) -> Result<Vec<TaskComment>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM task_comments WHERE task_id = ?1 ORDER BY created_at ASC"
        )?;
        let comments = stmt
            .query_map(params![task_id], |row| {
                Ok(TaskComment {
                    id: row.get("id")?,
                    task_id: row.get("task_id")?,
                    body: row.get("body")?,
                    author: row.get("author")?,
                    created_at: row.get("created_at")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(comments)
    }

    // ── Runs ────────────────────────────────────────────────────────────────────

    pub fn start_run(&self, task_id: &str, session_id: &str) -> Result<i64> {
        let now = Self::now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO task_runs (task_id, session_id, status, started_at) VALUES (?1, ?2, 'started', ?3)",
            params![task_id, session_id, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn complete_run(&self, run_id: i64, status: &str, summary: Option<&str>, metadata: Option<serde_json::Value>) -> Result<()> {
        let now = Self::now();
        let metadata_str = metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE task_runs SET status = ?1, completed_at = ?2, summary = ?3, metadata = ?4 WHERE id = ?5",
            params![status, now, summary, metadata_str, run_id],
        )?;
        Ok(())
    }

    pub fn get_runs(&self, task_id: &str) -> Result<Vec<TaskRun>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT * FROM task_runs WHERE task_id = ?1 ORDER BY started_at DESC"
        )?;
        let runs = stmt
            .query_map(params![task_id], |row| {
                let metadata_raw: Option<String> = row.get("metadata")?;
                Ok(TaskRun {
                    id: row.get("id")?,
                    task_id: row.get("task_id")?,
                    session_id: row.get("session_id")?,
                    status: row.get("status")?,
                    started_at: row.get("started_at")?,
                    completed_at: row.get("completed_at")?,
                    summary: row.get("summary")?,
                    metadata: metadata_raw.and_then(|m| serde_json::from_str(&m).ok()),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(runs)
    }

    // ── Link ───────────────────────────────────────────────────────────────────

    pub fn link_task(&self, task_id: &str, parent_id: &str) -> Result<()> {
        let task = self.get_task(task_id)?;
        let mut parent_ids = task.parent_ids;
        if !parent_ids.contains(&parent_id.to_string()) {
            parent_ids.push(parent_id.to_string());
            let parent_json = serde_json::to_string(&parent_ids)?;
            let now = Self::now();
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "UPDATE tasks SET parent_ids = ?1, updated_at = ?2 WHERE id = ?3",
                params![parent_json, now, task_id],
            )?;
        }
        self.add_event(task_id, "linked", serde_json::json!({"parent_id": parent_id}), "system")
    }
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let random: u64 = ((now * 0x517cc1b727220a95).wrapping_mul(0x853c49e6748fea9b)) as u64;
    format!("{:016x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (now & 0xffffffffffffffc0) as u64,
        (random >> 48) as u16,
        (random >> 44) as u16 & 0x0fff,
        (random >> 40) as u16 & 0x3fff | 0x8000,
        (now & 0xffffffffffff) as u64)
}

fn chrono_now() -> String {
    use chrono::Utc;
    Utc::now().to_rfc3339()
}