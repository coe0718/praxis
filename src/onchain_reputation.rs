//! Onchain Reputation System.
//!
//! Verifiable reputation from completed work, stored on-chain via smart contracts.

use serde::{Deserialize, Serialize};

/// Reputation event for onchain recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReputationEvent {
    /// Work completed successfully.
    WorkCompleted {
        work_id: String,
        rating: u32,
        timestamp: i64,
    },
    /// Payment received.
    PaymentReceived {
        amount: String,
        token: String,
        timestamp: i64,
    },
    /// Dispute filed.
    DisputeFiled {
        work_id: String,
        reason: String,
        timestamp: i64,
    },
}

/// Contract configuration.
#[derive(Debug, Clone)]
pub struct ContractConfig {
    pub address: String,
    pub abi: String,
    pub rpc_url: String,
}

/// Onchain reputation manager.
pub struct OnchainReputation {
    agent_id: String,
    events: Vec<ReputationEvent>,
    contract: Option<ContractConfig>,
}

impl OnchainReputation {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            events: Vec::new(),
            contract: None,
        }
    }

    /// Set contract configuration.
    pub fn set_contract(&mut self, config: ContractConfig) {
        self.contract = Some(config);
    }

    /// Record a reputation event.
    pub fn record_event(&mut self, event: ReputationEvent) {
        self.events.push(event);
    }

    /// Calculate reputation score.
    pub fn calculate_score(&self) -> u32 {
        let mut score: u32 = 100; // Base score

        for event in &self.events {
            match event {
                ReputationEvent::WorkCompleted { rating, .. } => {
                    score += rating;
                }
                ReputationEvent::PaymentReceived { .. } => {
                    score += 10;
                }
                ReputationEvent::DisputeFiled { .. } => {
                    score = score.saturating_sub(20);
                }
            }
        }

        score.min(1000) // Cap at 1000
    }

    /// Get summary for onchain submission.
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({
            "agent_id": self.agent_id,
            "score": self.calculate_score(),
            "total_events": self.events.len(),
            "contract": self.contract.as_ref().map(|c| &c.address),
        })
    }
}

impl Default for OnchainReputation {
    fn default() -> Self {
        Self::new("default".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_score() {
        let mut rep = OnchainReputation::new("agent1".into());
        rep.record_event(ReputationEvent::WorkCompleted {
            work_id: "w1".into(),
            rating: 5,
            timestamp: 0,
        });
        rep.record_event(ReputationEvent::PaymentReceived {
            amount: "100".into(),
            token: "USDC".into(),
            timestamp: 0,
        });

        assert!(rep.calculate_score() >= 115);
    }

    #[test]
    fn test_dispute_penalty() {
        let mut rep = OnchainReputation::new("agent1".into());
        rep.record_event(ReputationEvent::WorkCompleted {
            work_id: "w1".into(),
            rating: 5,
            timestamp: 0,
        });
        rep.record_event(ReputationEvent::DisputeFiled {
            work_id: "w1".into(),
            reason: "late".into(),
            timestamp: 0,
        });

        assert!(rep.calculate_score() >= 85 && rep.calculate_score() < 115);
    }
}