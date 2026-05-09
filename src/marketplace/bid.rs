//! Bid management for marketplace work items.

use super::work_item::BidQuote;
use serde::{Deserialize, Serialize};

/// A bid on a work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// Work item being bid on.
    pub work_item_id: String,
    /// Agent submitting the bid.
    pub agent_id: String,
    /// Quote details.
    pub quote: BidQuote,
    /// Current bid status.
    pub status: BidStatus,
    /// When bid was submitted (unix timestamp).
    pub submitted_at: u64,
    /// When work started (if accepted).
    pub started_at: Option<u64>,
    /// When work completed.
    pub completed_at: Option<u64>,
}

impl Bid {
    /// Create a new bid.
    pub fn new(work_item_id: &str, agent_id: &str, quote: BidQuote) -> Self {
        Self {
            work_item_id: work_item_id.to_string(),
            agent_id: agent_id.to_string(),
            quote,
            status: BidStatus::Pending,
            submitted_at: chrono::Utc::now().timestamp() as u64,
            started_at: None,
            completed_at: None,
        }
    }

    /// Accept this bid for work.
    pub fn accept(&mut self) {
        self.status = BidStatus::Accepted;
        self.started_at = Some(chrono::Utc::now().timestamp() as u64);
    }

    /// Mark bid as completed.
    pub fn complete(&mut self) {
        self.status = BidStatus::Completed;
        self.completed_at = Some(chrono::Utc::now().timestamp() as u64);
    }
}

/// Status of a bid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BidStatus {
    /// Bid submitted, waiting for acceptance.
    Pending,
    /// Accepted - agent should start work.
    Accepted,
    /// In progress.
    InProgress,
    /// Completed successfully.
    Completed,
    /// Rejected by requester.
    Rejected,
    /// Cancelled.
    Cancelled,
}
