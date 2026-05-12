# Self-Update

> Download and swap Praxis binary at runtime — `praxis update` fetches the latest release from GitHub and swaps the binary atomically.

## Overview

The self-update module provides the ability to check for and perform binary updates at runtime. It queries the GitHub releases API for the latest release, compares the version tag against the current compiled version (`CARGO_PKG_VERSION`), and if a newer version is available, downloads the Linux binary asset, writes it to a temp location, makes it executable, copies the current binary as a backup, removes the old binary, and renames the new binary in place.

The update flow is designed for safety: binary is downloaded to a temp file first, verified (by existence), and only then is the current binary swapped out. A backup of the current binary is saved before removal.

**Current status:** Fully implemented. GitHub API integration, version comparison, binary download, and atomic swap are complete.

## Architecture

### Key Types

No significant types — the module provides two async public functions.

| Function | Description |
|----------|-------------|
| `check_for_updates` | Queries GitHub API, compares versions, returns available version tag. |
| `perform_self_update` | Downloads, verifies, and atomically swaps the binary. |

## Public API

```rust
/// Check for updates. Returns Some(tag) if a newer version is available.
pub async fn check_for_updates(paths: &PraxisPaths) -> Result<Option<String>>

/// Perform self-update: download, backup, swap.
pub async fn perform_self_update(paths: &PraxisPaths) -> Result<()>
```

### `check_for_updates` Flow

1. Queries `GET https://api.github.com/repos/coe0718/praxis/releases/latest` with `User-Agent: Praxis-Updater`.
2. Parses `tag_name` from JSON response.
3. Compares with `env!("CARGO_PKG_VERSION")`.
4. Returns `Some(tag)` if different, `None` if same version or API failure.

### `perform_self_update` Flow

1. Queries GitHub releases API for the latest release.
2. Finds the first asset whose name contains `"linux"`.
3. Downloads the binary bytes via `reqwest`.
4. Writes to `data_dir/praxis-new` (temp file).
5. Sets Unix permissions to `0o755` (executable).
6. Copies current executable to `data_dir/praxis-backup` (backup).
7. Removes current executable.
8. Renames `praxis-new` → current executable path (atomic swap).

## Safety

The update follows a safe swap pattern:

```
1. Download  → data_dir/praxis-new          (temp file)
2. chmod 755 → data_dir/praxis-new          (make executable)
3. Copy      → data_dir/praxis-backup       (backup current)
4. Remove    → current_exe                  (delete old binary)
5. Rename    → praxis-new → current_exe     (atomic swap)
```

In case of failure before step 5, the original binary is intact. In case of failure at steps 3-4, a backup exists at `data_dir/praxis-backup`.

## Configuration

No `praxis.toml` fields. Invoked via CLI:

```bash
# Check for updates
praxis update --check

# Perform update
praxis update --install
```

### Programmatic

```rust
use praxis::self_update::{check_for_updates, perform_self_update};

// Check for new version
if let Some(version) = check_for_updates(&paths).await? {
    println!("New version available: {}", version);

    // Perform the update
    perform_self_update(&paths).await?;
    println!("Update complete! Please restart Praxis.");
} else {
    println!("Already up to date.");
}
```

## Dependencies

- **`reqwest`** — HTTP client for GitHub API and binary download.
- **`serde_json`** — GitHub release API response parsing (`serde_json::Value`).
- **`tokio::fs`** — Async file operations (write, copy, remove, rename).
- **`PraxisPaths`** — Data directory for temp files and backups.

## Status

- ✅ GitHub releases API integration
- ✅ Version comparison against `CARGO_PKG_VERSION`
- ✅ Linux binary asset discovery (name contains "linux")
- ✅ Safe atomic binary swap with backup
- ✅ Unix executable permission setting (0o755)
- ⏳ Checksum verification of downloaded binary
- ⏳ Multi-platform support (macOS, Windows)
- ⏳ Rollback capability from backup binary

## Source

`src/self_update.rs`