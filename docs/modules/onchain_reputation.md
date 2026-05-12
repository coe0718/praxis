# Onchain Reputation

> Verifiable reputation from completed work, stored on-chain via smart contracts.

## Overview

The onchain_reputation module provides a framework for recording and scoring agent reputation events that can be submitted to a blockchain smart contract. It tracks three event types: `WorkCompleted` (with a rating), `PaymentReceived`, and `DisputeFiled`.

The `OnchainReputation` manager accumulates events, calculates a composite reputation score (base 100, capped at 1000), and generates a JSON summary suitable for on-chain submission. An optional `ContractConfig` holds the smart contract address, ABI, and RPC URL for future blockchain integration.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ReputationEvent` | Enum: `WorkCompleted { work_id, rating, timestamp }`, `PaymentReceived { amount, token, timestamp }`, `DisputeFiled { work_id, reason, timestamp }`. |
| `ContractConfig` | Smart contract configuration (address, ABI, RPC URL). |
| `OnchainReputation` | Event accumulator and score calculator. |

### Scoring Model

- **Base score:** 100
- **WorkCompleted:** +`rating` points
- **PaymentReceived:** +10 points
- **DisputeFiled:** −20 points (via `saturating_sub`)
- **Maximum:** 1000 (capped via `.min(1000)`)

## Public API

### `ReputationEvent`

```rust
pub enum ReputationEvent {
    WorkCompleted {
        work_id: String,
        rating: u32,
        timestamp: i64,
    },
    PaymentReceived {
        amount: String,
        token: String,
        timestamp: i64,
    },
    DisputeFiled {
        work_id: String,
        reason: String,
        timestamp: i64,
    },
}
```

### `ContractConfig`

```rust
pub struct ContractConfig {
    pub address: String,
    pub abi: String,
    pub rpc_url: String,
}
```

Smart contract deployment details. Not used for on-chain writes yet; designed as a scaffold for future `ethers-rs` or `alloy` integration.

### `OnchainReputation`

```rust
impl OnchainReputation {
    pub fn new(agent_id: String) -> Self
    pub fn set_contract(&mut self, config: ContractConfig)
    pub fn record_event(&mut self, event: ReputationEvent)
    pub fn calculate_score(&self) -> u32
    pub fn summary(&self) -> serde_json::Value
}
```

- **`new`** — Creates a new reputation tracker for the given agent ID. Starts with an empty event list and no contract config.
- **`set_contract`** — Configures the smart contract for eventual on-chain submission.
- **`record_event`** — Appends a reputation event to the internal list.
- **`calculate_score`** — Computes the reputation score from all recorded events (base 100, +rating per work, +10 per payment, −20 per dispute, capped at 1000).
- **`summary`** — Returns a JSON object with `agent_id`, `score`, `total_events`, and optional `contract` address.

### Summary JSON Output

```json
{
  "agent_id": "agent_001",
  "score": 115,
  "total_events": 2,
  "contract": "0xabc...123"
}
```

## Configuration

No TOML configuration. The agent ID and optional contract config are set programmatically.

## Usage

```rust
use praxis::onchain_reputation::{
    OnchainReputation, ReputationEvent, ContractConfig,
};

let mut rep = OnchainReputation::new("agent_001".into());

rep.record_event(ReputationEvent::WorkCompleted {
    work_id: "task_001".into(),
    rating: 5,
    timestamp: 1747000000,
});

rep.record_event(ReputationEvent::PaymentReceived {
    amount: "100".into(),
    token: "USDC".into(),
    timestamp: 1747000100,
});

let score = rep.calculate_score();
// 100 (base) + 5 (rating) + 10 (payment) = 115

let summary = rep.summary();
println!("Score: {score}, Summary: {summary}");

// Optionally configure on-chain submission
rep.set_contract(ContractConfig {
    address: "0xabc...123".into(),
    abi: "[...]".into(),
    rpc_url: "https://rpc.example.com".into(),
});
```

## Data Files

None. Events are held in-memory. Callers should serialise the event list to a file (e.g., `reputation.jsonl`) for persistence.

## Dependencies

- **`serde` / `serde_json`** — Serialization of reputation events and summary output.

## Source

`src/onchain_reputation.rs`