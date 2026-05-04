# Identity

> Agent identity management — SOUL.md, IDENTITY.md, GOALS.md parsing, and foundation document lifecycle.

## Overview

The `identity` module manages Praxis's sense of self. It handles three core responsibilities: **goal parsing** from `GOALS.md` (a structured Markdown format with checkbox syntax and metadata), **identity foundation management** (ensuring required identity files like `SOUL.md` and `IDENTITY.md` exist with valid initial content), and **session journaling** (appending session outcomes to a persistent journal and metrics file).

The module defines two key traits — `GoalParser` and `IdentityPolicy` — that abstract the parsing and lifecycle behaviors, with concrete implementations (`MarkdownGoalParser`, `LocalIdentityPolicy`) that work with the local filesystem. This design allows alternative implementations for testing or different storage backends.

## Architecture

### Core Types (`mod.rs`)

| Type | Description |
|------|-------------|
| `Goal` | A parsed goal: `id` (e.g. "G-001"), `title`, `completed` flag, `line_number`, `parent_id`, `blocked_by` list, `wake_when` condition. |
| `GoalParser` (trait) | `parse_goals(content)` and `load_goals(path)`. |
| `IdentityPolicy` (trait) | `ensure_foundation()`, `validate()`, `append_journal()`, `append_metrics()`, `read_day_count()`. |

### Goal Parsing (`goals.rs`)

| Type | Description |
|------|-------------|
| `MarkdownGoalParser` | Parses `GOALS.md` format: `- [ ] G-001: Title` with optional indented metadata. |
| `GoalPromotion` | Result of `ensure_goal()`: the goal ID and whether it was newly created. |

**Supported goal metadata** (indented lines below a goal):
- `parent: G-010` — hierarchical goal nesting.
- `blocked_by: G-001, external-api` — comma-separated dependency list.
- `wake_when: env:PRAXIS_GO` — condition that must be met before the agent works on this goal.

`ensure_goal()` is idempotent — it will create a goal if it doesn't exist, or return the existing one if the title matches.

### Identity Files (`files.rs`)

| Type | Description |
|------|-------------|
| `LocalIdentityPolicy` | Filesystem-based implementation of `IdentityPolicy`. |

**Responsibilities:**

1. **`ensure_foundation()`** — Creates the data directory structure and writes initial identity documents (SOUL.md, IDENTITY.md, etc.) if they don't exist. Never overwrites existing files.
2. **`validate()`** — Checks that all required identity files exist and `DAY_COUNT` is non-negative.
3. **`append_journal()`** — Appends a structured Markdown entry to the journal file with session outcome, goal, task, and summary.
4. **`append_metrics()`** — Appends a pipe-delimited row to the metrics file with session number, outcome, goal, task, and timestamp.
5. **`read_day_count()`** — Reads the current `DAY_COUNT` value from disk.

## Public API

```rust
// Goal types
pub struct Goal {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub line_number: usize,
    pub parent_id: Option<String>,
    pub blocked_by: Vec<String>,
    pub wake_when: Option<String>,
}

// Goal parser
pub struct MarkdownGoalParser;
impl GoalParser for MarkdownGoalParser {
    fn parse_goals(&self, content: &str) -> Result<Vec<Goal>>;
    fn load_goals(&self, path: &Path) -> Result<Vec<Goal>>;
}

// Goal promotion (used by learning module)
pub fn ensure_goal(path: &Path, title: &str) -> Result<GoalPromotion>;

// Identity policy
pub struct LocalIdentityPolicy;
impl IdentityPolicy for LocalIdentityPolicy {
    fn ensure_foundation(&self, paths: &PraxisPaths, config: &AppConfig, now: DateTime<Utc>) -> Result<()>;
    fn validate(&self, paths: &PraxisPaths) -> Result<()>;
    fn append_journal(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()>;
    fn append_metrics(&self, paths: &PraxisPaths, session: &StoredSession) -> Result<()>;
    fn read_day_count(&self, paths: &PraxisPaths) -> Result<i64>;
}
```

## Configuration

Identity files are managed by the `LocalIdentityPolicy` and created during `praxis init`. The initial content is templated from the `AppConfig` (agent name, timezone, security level).

### GOALS.md Format

```markdown
# Goals

- [ ] G-001: Ship the memory foundation
- [x] G-002: Already completed goal
- [ ] G-003: Dependent feature
  parent: G-001
  blocked_by: G-002, external-api
  wake_when: env:DEPLOY_READY
```

## Usage

```bash
# Initialize identity files (done during setup)
praxis init --name "Axonix" --timezone "UTC" --security-level 2

# Goals are managed in GOALS.md directly or via learning:
praxis learn accept --id 3   # Creates a new goal automatically
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `SOUL.md` | Markdown | Immutable core identity — operator-only, never agent-writable. |
| `IDENTITY.md` | Markdown | Evolving working identity — agent-writable. |
| `GOALS.md` | Markdown | Active goal list with checkbox syntax and metadata. |
| `AGENTS.md` | Markdown | Conventions and quality gate guidance. |
| `DAY_COUNT` | Plain text | Single integer tracking operational days. |
| `JOURNAL.md` | Markdown | Append-only session journal. |
| `METRICS.md` | Markdown | Pipe-delimited session metrics table. |

## Dependencies

- `config` — `AppConfig` for identity templating
- `paths` — `PraxisPaths` for all file locations
- `storage` — `StoredSession` for journal/metrics entries
- `chrono`, `serde`, `anyhow`

## Source

`src/identity/` — `mod.rs`, `goals.rs`, `files.rs`
