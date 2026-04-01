use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::Path,
};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{Event, EventSink};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventRecord {
    emitted_at: String,
    event: Event,
}

#[derive(Debug, Clone)]
pub struct FileEventSink {
    path: std::path::PathBuf,
}

impl FileEventSink {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
}

impl EventSink for FileEventSink {
    fn emit(&self, event: &Event) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("failed to open {}", self.path.display()))?;
        let record = EventRecord {
            emitted_at: Utc::now().to_rfc3339(),
            event: event.clone(),
        };
        let line = serde_json::to_string(&record).context("failed to serialize event")?;
        writeln!(file, "{line}")
            .with_context(|| format!("failed to append {}", self.path.display()))
    }
}

pub fn read_events_since(path: &Path, offset: u64) -> Result<(Vec<Event>, u64)> {
    if !path.exists() {
        return Ok((Vec::new(), offset));
    }

    let mut file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    file.seek(SeekFrom::Start(offset))
        .with_context(|| format!("failed to seek {}", path.display()))?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut events = Vec::new();
    let mut current_offset = offset;

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if bytes == 0 {
            break;
        }
        current_offset += bytes as u64;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str::<EventRecord>(trimmed)
            .with_context(|| format!("invalid event record in {}", path.display()))?;
        events.push(record.event);
    }

    Ok((events, current_offset))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::events::{Event, EventSink};

    use super::{FileEventSink, read_events_since};

    #[test]
    fn writes_and_reads_event_logs() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("events.jsonl");
        let sink = FileEventSink::new(path.clone());
        sink.emit(&Event {
            kind: "agent:test".to_string(),
            detail: "hello".to_string(),
        })
        .unwrap();

        let (events, offset) = read_events_since(&path, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert!(offset > 0);
        assert_eq!(events[0].kind, "agent:test");
    }
}
