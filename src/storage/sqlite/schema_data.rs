pub(super) const SCHEMA_VERSION: i64 = 12;

pub(super) const BASE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY,
    day INTEGER NOT NULL,
    session_num INTEGER NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT NOT NULL,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    goals_completed INTEGER NOT NULL DEFAULT 0,
    goals_attempted INTEGER NOT NULL DEFAULT 0,
    lines_written INTEGER NOT NULL DEFAULT 0,
    memory_captures INTEGER NOT NULL DEFAULT 0,
    loop_guard_triggers INTEGER NOT NULL DEFAULT 0,
    reviewer_passes INTEGER NOT NULL DEFAULT 0,
    reviewer_failures INTEGER NOT NULL DEFAULT 0,
    repeated_reads_avoided INTEGER NOT NULL DEFAULT 0,
    phase_durations TEXT NOT NULL,
    outcome TEXT NOT NULL,
    selected_goal_id TEXT,
    selected_goal_title TEXT,
    selected_task TEXT,
    action_summary TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS hot_memories (
    id INTEGER PRIMARY KEY,
    content TEXT NOT NULL,
    summary TEXT,
    importance REAL NOT NULL DEFAULT 0.5,
    tags TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_accessed TEXT,
    access_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT
);
CREATE VIRTUAL TABLE IF NOT EXISTS hot_fts USING fts5(content, summary, tags);
CREATE TABLE IF NOT EXISTS cold_memories (
    id INTEGER PRIMARY KEY,
    content TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    tags TEXT NOT NULL DEFAULT '[]',
    source_ids TEXT NOT NULL DEFAULT '[]',
    contradicts TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_reinforced TEXT
);
CREATE VIRTUAL TABLE IF NOT EXISTS cold_fts USING fts5(content, tags);
CREATE TABLE IF NOT EXISTS approval_requests (
    id INTEGER PRIMARY KEY,
    tool_name TEXT NOT NULL,
    summary TEXT NOT NULL,
    requested_by TEXT NOT NULL,
    write_paths TEXT NOT NULL DEFAULT '[]',
    payload_json TEXT,
    status TEXT NOT NULL,
    status_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS review_runs (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL,
    goal_id TEXT,
    status TEXT NOT NULL,
    summary TEXT NOT NULL,
    findings_json TEXT NOT NULL DEFAULT '[]',
    reviewed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS eval_runs (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL,
    eval_id TEXT NOT NULL,
    eval_name TEXT NOT NULL,
    status TEXT NOT NULL,
    severity TEXT NOT NULL,
    summary TEXT NOT NULL,
    evaluated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS session_snapshots (
    id INTEGER PRIMARY KEY,
    session_id INTEGER,
    session_started_at TEXT NOT NULL,
    phase TEXT NOT NULL,
    checkpoint TEXT NOT NULL,
    state_json TEXT NOT NULL,
    recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS provider_attempts (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL,
    phase TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    success INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_micros INTEGER NOT NULL DEFAULT 0,
    error TEXT
);
CREATE TABLE IF NOT EXISTS token_ledger (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL,
    phase TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    estimated_cost_micros INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS anatomy_index (
    path TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    token_estimate INTEGER NOT NULL,
    last_modified_at TEXT NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE IF NOT EXISTS do_not_repeat (
    id INTEGER PRIMARY KEY,
    statement TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    source_session_id INTEGER,
    severity TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TEXT
);
CREATE TABLE IF NOT EXISTS known_bugs (
    id INTEGER PRIMARY KEY,
    signature TEXT NOT NULL,
    symptoms TEXT NOT NULL,
    fix_summary TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    source_session_id INTEGER,
    resolved_at TEXT,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS learning_sources (
    path TEXT PRIMARY KEY,
    last_modified_at TEXT NOT NULL,
    byte_len INTEGER NOT NULL,
    summary TEXT NOT NULL,
    last_processed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS learning_runs (
    id INTEGER PRIMARY KEY,
    processed_sources INTEGER NOT NULL,
    changed_sources INTEGER NOT NULL,
    opportunities_created INTEGER NOT NULL,
    notes_json TEXT NOT NULL DEFAULT '[]',
    completed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS memory_links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_memory_id INTEGER NOT NULL,
    to_memory_id INTEGER NOT NULL,
    link_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(from_memory_id, to_memory_id, link_type)
);
CREATE TABLE IF NOT EXISTS decision_receipts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_started_at TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    goal_id TEXT,
    chosen_action TEXT NOT NULL,
    context_sources_json TEXT NOT NULL DEFAULT '[]',
    confidence REAL NOT NULL DEFAULT 0.0,
    approval_required INTEGER NOT NULL DEFAULT 0,
    recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS opportunities (
    id INTEGER PRIMARY KEY,
    signature TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL,
    evidence_json TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL,
    goal_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

pub(super) const EXPECTED_TABLES: &[&str] = &[
    "sessions",
    "hot_memories",
    "hot_fts",
    "cold_memories",
    "cold_fts",
    "approval_requests",
    "review_runs",
    "eval_runs",
    "session_snapshots",
    "provider_attempts",
    "token_ledger",
    "anatomy_index",
    "do_not_repeat",
    "known_bugs",
    "learning_sources",
    "learning_runs",
    "memory_links",
    "decision_receipts",
    "opportunities",
];
