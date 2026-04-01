mod file;

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub use file::{FileEventSink, read_events_since};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub kind: String,
    pub detail: String,
}

pub trait EventSink {
    fn emit(&self, event: &Event) -> Result<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEventSink;

impl EventSink for NoopEventSink {
    fn emit(&self, _event: &Event) -> Result<()> {
        Ok(())
    }
}
