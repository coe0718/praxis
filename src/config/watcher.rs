//! File-system watcher for `praxis.toml` hot-reload.
//!
//! Spawned by the daemon at startup.  When the config file is written,
//! the watcher sets a shared flag that the daemon loop checks each cycle.
//! The daemon then re-reads and validates the config before the next
//! session — no restart required.

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// A background watcher that detects writes to the Praxis config file.
pub struct ConfigWatcher {
    /// Set to `true` when the config file has been modified since the last read.
    pub dirty: Arc<AtomicBool>,
    /// Toggled to tell the background watcher to shut down.
    stop: Arc<AtomicBool>,
}

impl ConfigWatcher {
    /// Spawn a background watcher for `config_path`.
    ///
    /// Returns immediately.  The watcher runs on a dedicated thread (the `notify`
    /// crate handles the event loop internally) and sets `dirty` whenever the
    /// config file is created or written.
    pub fn spawn(config_path: PathBuf) -> Result<Self> {
        let dirty = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));

        let dirty_flag = Arc::clone(&dirty);
        let stop_flag = Arc::clone(&stop);

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    // Debounce: we only care about writes / creates.
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        dirty_flag.store(true, Ordering::Release);
                    }
                }
            },
            NotifyConfig::default().with_poll_interval(Duration::from_secs(2)),
        )
        .context("failed to create filesystem watcher")?;

        // Watch the parent directory so we catch atomic-rename writes
        // (editor save → new inode) that the daemon already handles.
        let watch_dir =
            config_path.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf();

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .context("failed to watch config directory")?;

        // Leak the watcher onto a thread — `notify` requires the watcher to
        // stay alive.  We shut it down by dropping the returned handle.
        std::thread::spawn(move || {
            // Block until told to stop or the watcher channel closes.
            while !stop_flag.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(500));
            }
            drop(watcher);
        });

        Ok(Self { dirty, stop })
    }

    /// Atomically read and clear the dirty flag.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }
}

impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}
