# Backup Archives

> Create portable snapshots of Praxis state — `praxis backup` creates verifiable portable archives, `praxis restore` restores from backup.

## Overview

The backup module provides async functions for creating, verifying, and restoring backup archives of the Praxis data directory. Currently a minimal implementation that creates a placeholder backup file and verifies integrity as a stub. The intended design uses `tar.gz` archives of important directories within the data directory.

**Current status:** Placeholder implementation. Core API surface is defined; actual tar.gz creation and integrity verification are future work.

## Architecture

### Key Types

No significant types — the module is function-oriented.

| Function | Description |
|----------|-------------|
| `create_backup` | Creates a backup archive (currently placeholder). |
| `verify_backup` | Verifies archive integrity (currently returns `true`). |
| `restore_backup` | Restores from a backup archive (currently no-op). |

## Public API

```rust
/// Create a backup archive at data_dir/praxis_backup_{timestamp}.tar.gz
pub async fn create_backup(paths: &PraxisPaths) -> Result<PathBuf>

/// Verify a backup archive's integrity
pub fn verify_backup(backup_path: &PathBuf) -> Result<bool>

/// Restore from a backup archive
pub async fn restore_backup(paths: &PraxisPaths, backup_path: &PathBuf) -> Result<()>
```

- **`create_backup`** — Creates a backup directory under `data_dir/backups/` and writes a placeholder file named `praxis_backup_{YYYYMMDD_HHMMSS}.tar.gz`. Returns the path to the backup file.
- **`verify_backup`** — Stub: returns `Ok(true)` without actual integrity checking.
- **`restore_backup`** — Stub: returns `Ok(())` without extracting.

## Configuration

No `praxis.toml` fields. Invoked via CLI:

```bash
# Create backup
praxis backup

# Restore from backup
praxis restore ./praxis_backup_20250101_120000.tar.gz
```

## Dependencies

- **`anyhow`** — Error handling.
- **`tokio::fs`** — Async filesystem operations.
- **`chrono`** — Timestamp generation for backup filenames.
- **`PraxisPaths`** — Data directory location.

## Status

- ✅ API surface defined (create / verify / restore)
- ✅ Timestamped backup file naming
- ⏳ Real tar.gz archive creation
- ⏳ Integrity verification (checksums, structure)
- ⏳ Selective file/directory inclusion
- ⏳ Restore with safety checks (not overwriting running state)

## Source

`src/backup.rs`