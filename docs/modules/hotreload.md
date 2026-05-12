# Hotreload

> Auto-reload configuration on file change for zero-downtime config updates.

## Overview

The hotreload module provides a simple file-change watcher (`ConfigWatcher`) that monitors configuration files and signals when a reload is needed. This enables Praxis to pick up configuration changes without a full process restart, supporting zero-downtime config updates as part of the Moltis feature set.

The current implementation checks the config file by re-reading it on demand. A future extension could integrate `inotify`-based file system events for push-based notifications.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ConfigWatcher` | Watches a single config file path and exposes a `check_reload()` method. |

## Public API

### `ConfigWatcher`

```rust
pub struct ConfigWatcher {
    config_path: PathBuf,
}
```

```rust
impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> Self
    pub fn check_reload(&self) -> Result<bool>
}
```

- **`new`** — Creates a new watcher for the given config file path.
- **`check_reload`** — Reads the config file from disk and returns `Ok(true)`. Currently always reports that a reload is needed; the return value signals that the file was successfully read. Future versions will compare content hashes to detect actual changes.

## Configuration

No TOML configuration fields. The watcher is instantiated programmatically with a path:

```rust
let watcher = ConfigWatcher::new(paths.config_dir.join("praxis.toml"));
```

## Usage

```rust
use praxis::hotreload::ConfigWatcher;

let watcher = ConfigWatcher::new(config_path);

// In a loop or periodic tick:
if watcher.check_reload()? {
    // Re-read and re-apply configuration
    reload_config()?;
}
```

## Data Files

None. The watcher reads the target config file directly — no auxiliary files are created or maintained.

## Dependencies

- **`anyhow`** — Error handling.

## Source

`src/hotreload.rs`