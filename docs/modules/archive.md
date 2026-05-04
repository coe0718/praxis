# Archive

> Export/import bundles, audit exports, and daily snapshots for backup, migration, and compliance.

## Overview

The Archive module handles all data portability concerns for a Praxis instance. It provides three core capabilities:

1. **Bundle export/import** — Full data directory snapshots that can be moved between machines or used for disaster recovery. A bundle includes all files from the data directory, the SQLite database, session state, and a manifest describing the export.

2. **Audit export** — Human-readable Markdown reports summarizing recent sessions, approvals, reviews, evals, provider usage, memory writes, and events. Designed for compliance review or operator inspection.

3. **Daily snapshots** — Automatic, once-per-day bundle exports stored in a `backups/` directory with configurable retention. These are triggered by the Praxis runtime loop.

All exports use a consistent tree-copy engine that refuses to follow symlinks, normalizes relative paths, and prevents overlapping source/destination directories — protecting against both accidental data loss and path-traversal attacks.

## Architecture

### Module structure

| Submodule | Purpose |
|---|---|
| `mod.rs` | Re-exports, `ExportManifest` struct, format constants. |
| `audit.rs` | `export_audit()` — generates a Markdown audit report from the database. |
| `bundle.rs` | `export_bundle()` / `import_bundle()` — full data directory export and import. |
| `snapshots.rs` | `maybe_create_daily_snapshot()` — automated daily backup with retention pruning. |
| `tree.rs` | Low-level directory tree copy/restore with symlink rejection and path normalization. |

### Key types

**`ExportManifest`** — JSON manifest written to the root of every bundle. Contains format version, timestamps, source paths, schema version, and a file inventory.

**`BundleExportSummary`** / **`BundleImportSummary`** / **`SnapshotExportSummary`** / **`AuditExportSummary`** — Returned from export/import operations to report what was processed.

### Security measures

- Symlinks are refused during copy operations.
- Path traversal (`..`) is rejected during restoration.
- Source and destination paths are checked for overlap to prevent recursive copies.
- `safe_relative()` strips all non-normal path components.

## Public API

```rust
// Bundle export/import
export_bundle(config, paths, output_dir, overwrite, now) -> Result<BundleExportSummary>
import_bundle(data_dir, input_dir, overwrite) -> Result<BundleImportSummary>

// Audit export
export_audit(paths, output_file, overwrite, now, days) -> Result<AuditExportSummary>

// Daily snapshots
maybe_create_daily_snapshot(config, paths, now) -> Result<Option<SnapshotExportSummary>>
```

## Configuration

### `praxis.toml` fields

```toml
[runtime]
# Number of daily snapshots to retain. Older snapshots are pruned.
snapshot_retention_days = 7   # default: 7
```

### CLI flags

```bash
# Export bundle
praxis export state --output ./my-backup --overwrite

# Import bundle
praxis import --input ./my-backup --overwrite

# Export audit report (last 14 days)
praxis export audit --output audit-report.md --days 14 --overwrite
```

## Usage

### CLI commands

```bash
# Create a full export bundle
praxis export state --output /tmp/praxis-backup

# Import a bundle into a fresh data directory
praxis import --input /tmp/praxis-backup

# Export an audit report covering the last 30 days
praxis export audit --output audit.md --days 30

# Overwrite an existing export
praxis export state --output /tmp/praxis-backup --overwrite
```

### In code (daily snapshot)

```rust
if let Some(summary) = archive::maybe_create_daily_snapshot(&config, &paths, now)? {
    log::info!("daily snapshot created: {} files, pruned {} old snapshots",
        summary.data_file_count, summary.pruned);
}
```

## Data Files

### Exported bundle structure

```
output_dir/
├── manifest.json              # ExportManifest metadata
├── data/                      # Mirrored data directory contents
│   ├── praxis.toml
│   ├── SOUL.md
│   ├── GOALS.md
│   ├── ...
├── runtime/
│   ├── database.sqlite        # Copy of the SQLite database
│   └── session_state.json     # Copy of session state (if present)
```

### Local paths

| Path | Purpose |
|---|---|
| `{data_dir}/backups/` | Daily snapshot storage. |
| `{data_dir}/backups/snapshot-YYYYMMDD/` | Individual daily snapshot. |

## Dependencies

- **`paths`** — `PraxisPaths` for locating all data files.
- **`config`** — `AppConfig` for database path, state file, retention settings.
- **`storage`** — `SqliteSessionStore` for schema validation during import.
- **`identity`** — `LocalIdentityPolicy` validation during import.
- **`tools/registry`** — `FileToolRegistry` validation during import.
- **`tree`** (internal) — Safe directory copy/restore primitives.

## Source

`src/archive/` — `mod.rs`, `audit.rs`, `bundle.rs`, `snapshots.rs`, `tree.rs`
