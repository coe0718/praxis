//! Marketplace reputation system.

use serde::{Deserialize, Serialize};

/// Reputation record for an agent on a marketplace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketplaceReputation {
    /// Agent identifier.
    pub agent_id: String,
    /// Total completed jobs.
    pub jobs_completed: u64,
    /// Average rating (0-5 stars).
    pub avg_rating: f32,
    /// Total earned in USD cents.
    pub total_earned: u64,
    /// Success rate 0.0-1.0.
    pub success_rate: f32,
    /// History of reputation events.
    pub history: Vec<ReputationEvent>,
}

impl MarketplaceReputation {
    /// Create new reputation record.
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            ..Default::default()
        }
    }

    /// Record a completed job.
    pub fn record_completion(&mut self, rating: f32, amount: u64) {
        self.jobs_completed += 1;
        self.total_earned += amount;

        // Update average rating
        let total_rating = self.avg_rating * (self.jobs_completed - 1) as f32 + rating;
        self.avg_rating = total_rating / self.jobs_completed as f32;

        self.history.push(ReputationEvent {
            timestamp: chrono::Utc::now().timestamp() as u64,
            event_type: EventType::JobCompleted,
            delta: rating as i32,
            amount: Some(amount),
        });
    }

    /// Get reputation score for bidding (0-1000).
    pub fn bid_score(&self) -> u32 {
        let base = if self.jobs_completed == 0 { 100 } else { 0 };
        let rating_component = (self.avg_rating / 5.0 * 400.0) as u32;
        let volume_component = (self.jobs_completed.min(100) * 5) as u32;
        base + rating_component + volume_component
    }
}

/// A reputation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEvent {
    pub timestamp: u64,
    pub event_type: EventType,
    pub delta: i32,
    pub amount: Option<u64>,
}

/// Type of reputation event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    JobCompleted,
    JobCancelled,
    RatingReceived,
    PaymentVerified,
}
