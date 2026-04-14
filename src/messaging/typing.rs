//! Typing indicators — liveness signals to operators during active sessions.
//!
//! While a session is actively working, messaging adapters should emit typing
//! or presence indicators where the platform supports them.  The goal is to
//! show liveness without turning the agent into a status-spam bot.
//!
//! Adapters implement [`TypingIndicator`] and call `begin()` when a response
//! starts processing and `end()` when delivery is complete.  Adapters that
//! don't support typing indicators use [`NoopTypingIndicator`].

use anyhow::Result;

/// Signals that the agent is actively composing a response.
pub trait TypingIndicator: Send + Sync {
    /// Begin showing a typing indicator in the given conversation.
    fn begin(&self, conversation_id: &str) -> Result<()>;

    /// Stop the typing indicator in the given conversation.
    fn end(&self, conversation_id: &str) -> Result<()>;
}

/// A [`TypingIndicator`] that does nothing.
///
/// Used for adapters that don't support presence indicators, and as a default
/// in contexts where no messaging adapter is active.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopTypingIndicator;

impl TypingIndicator for NoopTypingIndicator {
    fn begin(&self, _conversation_id: &str) -> Result<()> {
        Ok(())
    }

    fn end(&self, _conversation_id: &str) -> Result<()> {
        Ok(())
    }
}

/// A [`TypingIndicator`] that logs calls for testing.
#[cfg(test)]
pub struct RecordingTypingIndicator {
    pub calls: std::sync::Mutex<Vec<(String, String)>>,
}

#[cfg(test)]
impl RecordingTypingIndicator {
    pub fn new() -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl TypingIndicator for RecordingTypingIndicator {
    fn begin(&self, conversation_id: &str) -> Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(("begin".to_string(), conversation_id.to_string()));
        Ok(())
    }

    fn end(&self, conversation_id: &str) -> Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(("end".to_string(), conversation_id.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{NoopTypingIndicator, RecordingTypingIndicator, TypingIndicator};

    #[test]
    fn noop_never_errors() {
        let t = NoopTypingIndicator;
        t.begin("chat:42").unwrap();
        t.end("chat:42").unwrap();
    }

    #[test]
    fn recording_indicator_captures_calls() {
        let t = RecordingTypingIndicator::new();
        t.begin("chat:42").unwrap();
        t.end("chat:42").unwrap();
        t.begin("chat:99").unwrap();

        let calls = t.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], ("begin".to_string(), "chat:42".to_string()));
        assert_eq!(calls[1], ("end".to_string(), "chat:42".to_string()));
        assert_eq!(calls[2], ("begin".to_string(), "chat:99".to_string()));
    }
}
