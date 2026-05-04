//! (#29) Live model switching via a thread-safe override cell.
//!
//! The `ModelOverride` provides a process-wide, thread-safe mechanism for
//! switching models mid-session without modifying configuration files.  The
//! override takes precedence over both `model_pin` and the file-based
//! `model_override` used by the `praxis model` CLI.
//!
//! ## Hierarchy (highest priority wins)
//!
//! 1. `ModelOverride` (in-memory, set via `/model` REPL command or API)
//! 2. `model_override` file (set via `praxis model <name>` CLI)
//! 3. `agent.model_pin` in `praxis.toml`
//! 4. Default model from provider settings

use std::sync::{Arc, Mutex};

/// Thread-safe cell holding an optional model override string.
#[derive(Debug, Clone, Default)]
pub struct ModelOverride {
    inner: Arc<Mutex<Option<String>>>,
}

impl ModelOverride {
    /// Create a new empty override cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the model override.  Replaces any previous override.
    pub fn set(&self, model: impl Into<String>) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(model.into());
    }

    /// Clear the override, reverting to the configured model.
    pub fn clear(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
    }

    /// Read the current override, if any.
    pub fn get(&self) -> Option<String> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }
}

/// Process-wide model override singleton.
/// Used by the chat REPL `/model` command and the API to switch models
/// without rebuilding the backend.
use std::sync::OnceLock;

static GLOBAL_MODEL_OVERRIDE: OnceLock<ModelOverride> = OnceLock::new();

/// Returns a reference to the global model override cell.
pub fn global_model_override() -> &'static ModelOverride {
    GLOBAL_MODEL_OVERRIDE.get_or_init(ModelOverride::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_clear_roundtrip() {
        let cell = ModelOverride::new();
        assert!(cell.get().is_none());

        cell.set("claude-3-5-sonnet-latest");
        assert_eq!(cell.get().as_deref(), Some("claude-3-5-sonnet-latest"));

        cell.clear();
        assert!(cell.get().is_none());
    }

    #[test]
    fn clone_shares_state() {
        let cell = ModelOverride::new();
        let clone = cell.clone();
        cell.set("gpt-4o");

        assert_eq!(clone.get().as_deref(), Some("gpt-4o"));
    }
}
