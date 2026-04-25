//! Clarify tool — lets the agent ask the operator clarifying questions
//! during a session when it encounters ambiguity.
//!
//! The clarify tool publishes a question event to the message bus and
//! blocks until the operator responds. The response is returned as the
//! tool execution result so the agent can incorporate it into its next action.

use std::{fs, path::Path, thread, time::Duration};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::bus::{BusEvent, FileBus, MessageBus};

/// A clarifying question the agent wants to ask the operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyQuestion {
    /// Unique ID for this question, used to match the response.
    pub id: String,
    /// The question text to present to the operator.
    pub question: String,
    /// Optional preset choices (if empty, any free-form answer is accepted).
    #[serde(default)]
    pub choices: Vec<String>,
    /// Timestamp when the question was asked.
    pub asked_at: String,
}

/// The operator's response to a clarifying question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifyResponse {
    /// Matches the question ID.
    pub question_id: String,
    /// The operator's answer.
    pub answer: String,
    /// Timestamp when the response was received.
    pub responded_at: String,
}

/// How long to wait for an operator response (seconds).
const CLARIFY_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// How long to sleep between polling the bus for a response.
const POLL_INTERVAL_MS: u64 = 1000;

/// Execute a clarify action: ask a question and wait for the operator's response.
///
/// Returns the operator's answer text or an error if timed out.
pub fn execute_clarify(bus_file: &Path, question: &str, choices: &[String]) -> Result<String> {
    let bus = FileBus::new(bus_file);
    let id = format!(
        "clarify-{}",
        std::time::UNIX_EPOCH.elapsed().map(|d| d.as_millis()).unwrap_or(0)
    );
    let asked_at = chrono::Utc::now().to_rfc3339();

    let clarify = ClarifyQuestion {
        id: id.clone(),
        question: question.to_string(),
        choices: choices.to_vec(),
        asked_at,
    };

    let payload =
        serde_json::to_string(&clarify).context("failed to serialize clarify question")?;

    let event = BusEvent::new("clarify", "praxis", "operator", "praxis", payload);

    bus.publish(&event).context("failed to publish clarify event to bus")?;

    log::info!("clarify: asked question '{id}'. Waiting for operator response...");

    // Poll the bus for a response.
    let deadline = std::time::Instant::now() + Duration::from_secs(CLARIFY_TIMEOUT_SECS);
    loop {
        if std::time::Instant::now() >= deadline {
            bail!("clarify: operator did not respond within {} seconds", CLARIFY_TIMEOUT_SECS);
        }

        // Read the clarify response file (operator writes their answer here).
        let response_path = bus_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("clarify_response.json");

        if let Ok(raw) = fs::read_to_string(&response_path)
            && let Ok(response) = serde_json::from_str::<ClarifyResponse>(&raw)
                && response.question_id == id {
                    // Consume the response file.
                    let _ = fs::remove_file(&response_path);
                    log::info!("clarify: received response for '{}': {}", id, response.answer);
                    return Ok(format!("operator answered: {}", response.answer));
                }

        // Also check the bus for clarify_response events.
        if let Ok(events) = bus.drain() {
            for event in events {
                if event.kind == "clarify_response"
                    && let Ok(response) = serde_json::from_str::<ClarifyResponse>(&event.payload)
                        && response.question_id == id {
                            log::info!("clarify: received bus response for '{}'", id);
                            return Ok(format!("operator answered: {}", response.answer));
                        }
                // Re-publish non-clarify events we consumed.
                let _ = bus.publish(&event);
            }
        }

        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
    }
}
