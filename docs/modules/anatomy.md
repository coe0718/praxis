# Anatomy

> Auto-generated file summaries and token estimates for the agent's knowledge index

## Overview

The `anatomy` module maintains a lightweight index of every file the agent reads — identity files (`SOUL.md`, `IDENTITY.md`), tool manifests, and any other markdown or TOML files in the data directory. Each entry records the file path, a one-line description, an estimated token count, the last-modified timestamp, and optional tags.

This index serves two purposes:

1. **Context deduplication** — When the `TrackedContextReader` (in `context::files`) encounters a file that has already been read in the current session with the same modification time, it replaces the full content with a compact anatomy summary ("path: description (~N tokens)"), saving precious context tokens.

2. **Stale file detection** — `refresh_stale_anatomy()` compares on-disk modification times against the stored index and re-indexes files that have changed. This runs during idle/maintenance windows to keep `CAPABILITIES.md` in sync without blocking active sessions.

Token estimation uses a simple heuristic: character count divided by 4 (rounded up). This is a rough approximation of BPE tokenization but is fast, deterministic, and sufficient for budget planning.

## Architecture

### Key Types

- **`NewAnatomyEntry`** — A fresh index entry ready for upsert. Fields: `path`, `description`, `token_estimate`, `last_modified_at`, `tags`.
- **`StoredAnatomyEntry`** — The persisted form (identical fields to `NewAnatomyEntry`).

### Key Functions

| Function | Purpose |
|---|---|
| `build_entry(path, content, tags)` | Create a `NewAnatomyEntry` from a file's content and metadata |
| `render_summary(path, description, token_estimate)` | Format a one-line summary string |
| `refresh_stale_anatomy(paths)` | Scan identity and tool files, re-index any that changed on disk |
| `describe(path, content)` | Extract a description: uses the first `# heading` line, or the first 80 chars of the first non-empty line, or the filename |
| `estimate_tokens(content)` | `chars / 4` (rounded up) |
| `modified_at(path)` | Read file mtime as RFC 3339 |

## Public API

```rust
pub fn build_entry(path: &Path, content: &str, tags: Vec<String>) -> Result<NewAnatomyEntry>
pub fn render_summary(path: &str, description: &str, token_estimate: i64) -> String
pub fn refresh_stale_anatomy(paths: &PraxisPaths) -> Result<usize>
```

## Configuration

No direct configuration. Anatomy entries are stored via the `AnatomyStore` trait (implemented by `SqliteSessionStore`).

## Usage

### Automatic

Anatomy indexing happens automatically:
- During **Orient**, `TrackedContextReader::read()` calls `build_entry()` and `upsert_anatomy_entry()` for every file loaded into context.
- During **idle maintenance**, `refresh_stale_anatomy()` re-indexes changed files.

### CLI

```bash
# View the current status (includes anatomy entry count)
praxis status
```

## Data Files

Anatomy entries are stored in the SQLite database (table: `anatomy_entries`), accessed via `AnatomyStore::upsert_anatomy_entry()` and `AnatomyStore::anatomy_last_modified()`.

The indexed files come from:
- `PraxisPaths::identity_files()` — SOUL.md, IDENTITY.md, GOALS.md, AGENTS.md, PATTERNS.md, JOURNAL.md, etc.
- `PraxisPaths::tools_dir` — All `.toml` and `.json` tool manifest files

## Dependencies

- `storage` — `AnatomyStore`, `SqliteSessionStore`
- `paths` — `PraxisPaths`

## Source

`src/anatomy.rs`
