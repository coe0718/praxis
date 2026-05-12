# Edit Checkpoints

> Auto-snapshot files before agent modifies them — provides rollback capability if changes are problematic.

## Overview

The checkpoints module provides automatic file snapshotting before the agent performs edit operations. A `CheckpointManager` creates SHA-256 checksummed copies of files before modification, stored in a `checkpoints/` directory under the data directory. If a subsequent operation fails, the `with_checkpoint` wrapper automatically restores the original file from the checkpoint.

Each checkpoint is stored as a file named `{timestamp}_{first_8_chars_of_checksum}` in the snapshots directory, enabling easy identification and potential cleanup of old checkpoints.

**Current status:** Fully implemented. Snapshot creation, restoration, and the auto-restore-on-error pattern are complete.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `CheckpointManager` | Creates and restores checkpoints for file operations. |
| `Checkpoint` | Record of a single checkpoint: original path, SHA-256 checksum, timestamp. |

### Relationships

`CheckpointManager` is constructed with `PraxisPaths` to determine the snapshots directory. `Checkpoint` records point back to the original file path.

## Public API

### `CheckpointManager`

```rust
impl CheckpointManager {
    pub fn new(paths: &PraxisPaths) -> Self
    pub fn checkpoint(&self, path: &PathBuf) -> Result<Checkpoint>
    pub fn restore(&self, checkpoint: &Checkpoint) -> Result<()>
}
```

- **`checkpoint`** — Computes SHA-256 checksum of the file at `path`, creates a copy in the snapshots directory, and returns a `Checkpoint` record.
- **`restore`** — Copies the snapshot file back to the original path (if the snapshot exists).

### `Checkpoint`

```rust
pub struct Checkpoint {
    pub original_path: PathBuf,
    pub checksum: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### Free Function

```rust
pub async fn with_checkpoint<F>(paths: &PraxisPaths, path: &PathBuf, f: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
```

- **`with_checkpoint`** — Creates a checkpoint before running `f()`. If `f()` returns an error, automatically restores the original file from the checkpoint. Returns the same error after restoration.

## Configuration

No `praxis.toml` fields. Checkpoints are stored under `data_dir/checkpoints/`:

```
data_dir/
  checkpoints/
    20250101_120000_abc12345
    20250101_121500_def67890
```

### Programmatic

```rust
use praxis::checkpoints::{CheckpointManager, with_checkpoint};
use std::path::PathBuf;

let paths = PraxisPaths::from_data_dir("/path/to/data");

// Manual checkpointing
let mgr = CheckpointManager::new(&paths);
let cp = mgr.checkpoint(&PathBuf::from("/path/to/file.md"))?;
// ... edit file ...
if needs_rollback {
    mgr.restore(&cp)?;
}

// Auto-checkpoint with error recovery
with_checkpoint(&paths, &PathBuf::from("/path/to/file.md"), || {
    // Edit the file
    std::fs::write("/path/to/file.md", "new content")?;
    Ok(())
}).await?;  // Auto-restores on error
```

## Dependencies

- **`sha2`** — SHA-256 checksum for file identity and snapshot naming.
- **`chrono`** — Timestamps for snapshot filenames.
- **`PraxisPaths`** — Data directory location for snapshots directory.

## Status

- ✅ Checkpoint creation with SHA-256 checksum
- ✅ File restoration from checkpoint
- ✅ Auto-restore on error via `with_checkpoint`
- ✅ Deterministic snapshot filenames (timestamp + checksum prefix)
- ⏳ Checkpoint retention/cleanup policy
- ⏳ Checkpoint listing and management CLI

## Source

`src/checkpoints.rs`