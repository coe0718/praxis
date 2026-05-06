//! Session spawning — programmatic creation of new agent sessions.
//!
//! #7 Sessions spawn (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! The kanban dispatcher needs to create new sessions for worker tasks.
//! This module provides `SessionSpawner` that writes a `WakeIntent` to the
//! bus, enabling the daemon to pick up the new session on its next poll cycle.

pub mod spawn;

pub use spawn::{SessionSpawnRequest, SessionSpawnResult, SessionSpawner, SpawnStatus};
