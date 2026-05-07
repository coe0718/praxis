//! Work item representation for marketplace tasks.

use serde::{Deserialize, Serialize};

/// A unit of work available on the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    /// Unique identifier from marketplace.
    pub id: String,
    /// Title/description of the work.
    pub title: String,
    /// Detailed requirements/specification.
    pub spec: String,
    /// Maximum payout in USD cents.
    pub max_price: u64,
    /// Required capabilities/tags.
    pub required_caps: Vec<String>,
    /// Time limit in seconds.
    pub deadline_secs: Option<u64>,
    /// Whether this is recurring.
    pub recurring: bool,
}

/// Quote/bid information from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidQuote {
    /// Proposed price in USD cents.
    pub price: u64,
    /// Estimated completion time in seconds.
    pub eta_secs: u64,
    /// Confidence score 0.0-1.0.
    pub confidence: f32,
    /// Why the agent is qualified.
    pub qualification: String,
}

/// Status of a work item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkItemStatus {
    /// Open for bidding.
    Open,
    /// In progress by assigned agent.
    InProgress,
    /// Completed and paid.
    Completed,
    /// Cancelled.
    Cancelled,
}