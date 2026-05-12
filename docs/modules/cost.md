# Cost Tracker

> Track API costs per session, tool, and provider — records token usage and estimated costs for LLM and API calls.

## Overview

The cost tracker module records every API call with its token counts, model, provider, and session association. Costs are computed using known per-model pricing (per million tokens) in USD. The module ships with built-in pricing tables for OpenAI, Anthropic, and Google models, plus embedding, TTS, and STT models. Custom pricing can be overridden for any model.

Cost entries are stored in a `Vec<CostEntry>` in memory and provide session-level, tool-level, and model-level aggregation. Token usage summaries are also available.

**Current status:** Fully implemented. Used in the LLM backend to record costs after each completion.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ModelPricing` | Cost per million input/output tokens (USD). |
| `CostEntry` | A single recorded API call: timestamp, session, model, provider, tokens, USD cost, tool. |
| `CostTracker` | In-memory entry storage with aggregation queries and custom pricing overrides. |

### Built-in Pricing

| Model | Input ($/M) | Output ($/M) |
|-------|-------------|--------------|
| gpt-4o | 2.50 | 10.00 |
| gpt-4o-mini | 0.15 | 0.60 |
| gpt-4.1 | 2.00 | 8.00 |
| gpt-4.1-mini | 0.40 | 1.60 |
| gpt-4.1-nano | 0.10 | 0.40 |
| o3 | 10.00 | 40.00 |
| o4-mini | 1.50 | 6.00 |
| claude-sonnet-4 | 3.00 | 15.00 |
| claude-haiku-3.5 | 0.80 | 4.00 |
| gemini-2.5-pro | 1.25 | 10.00 |
| gemini-2.5-flash | 0.15 | 0.60 |
| text-embedding-3-small | 0.02 | 0.00 |
| text-embedding-3-large | 0.13 | 0.00 |
| tts-1 | 15.00 | 0.00 |
| whisper-1 | 6.00 | 0.00 |

Unknown models default to $1.00/M input, $3.00/M output.

## Public API

### `ModelPricing`

```rust
pub struct ModelPricing {
    pub input_per_m: f64,
    pub output_per_m: f64,
}

impl ModelPricing {
    pub fn calculate(&self, input_tokens: u64, output_tokens: u64) -> f64
}
```

### `CostEntry`

```rust
pub struct CostEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub session_id: String,
    pub model: String,
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub tool: Option<String>,
}
```

### `CostTracker`

```rust
impl CostTracker {
    pub fn new() -> Self
    pub fn set_pricing(&mut self, model: &str, pricing: ModelPricing)
    pub fn record(&mut self, session_id: &str, model: &str, provider: &str,
                  input_tokens: u64, output_tokens: u64, tool: Option<&str>) -> f64
    pub fn total_cost(&self) -> f64
    pub fn session_cost(&self, session_id: &str) -> f64
    pub fn tool_cost(&self, tool: &str) -> f64
    pub fn cost_by_model(&self) -> HashMap<String, f64>
    pub fn token_summary(&self) -> (u64, u64)
    pub fn len(&self) -> usize
    pub fn is_empty(&self) -> bool
    pub fn entries(&self) -> &[CostEntry]
}
```

- **`set_pricing`** — Override pricing for any model (custom > known > default).
- **`record`** — Records a cost entry; returns the computed USD cost.
- **`session_cost`** — Total cost for a specific session.
- **`tool_cost`** — Total cost for a specific tool.
- **`cost_by_model`** — Map of model → total cost.
- **`token_summary`** — Returns `(total_input_tokens, total_output_tokens)`.

### Free Functions

```rust
pub fn known_pricing() -> HashMap<String, ModelPricing>
```

Returns the built-in pricing table for all known models.

## Configuration

No `praxis.toml` fields directly. The tracker is used programmatically:

```rust
use praxis::cost::{CostTracker, ModelPricing};

let mut tracker = CostTracker::new();

// Set custom pricing for a model
tracker.set_pricing("my-model", ModelPricing {
    input_per_m: 1.0,
    output_per_m: 2.0,
});

// Record an API call
let cost = tracker.record("session-123", "gpt-4o", "openai", 1500, 400, Some("code_gen"));
println!("Cost: ${:.6}", cost);

// Get aggregate stats
println!("Total cost: ${:.4}", tracker.total_cost());
println!("Tokens: {} in / {} out", tracker.token_summary().0, tracker.token_summary().1);
```

## Dependencies

- **`chrono`** — Timestamps for cost entries.
- **`HashMap`** — Pricing tables and model breakdowns.

## Status

- ✅ Built-in pricing for 15+ models across OpenAI, Anthropic, Google
- ✅ Custom pricing overrides (custom > known > default)
- ✅ Session-level and tool-level cost aggregation
- ✅ Cost breakdown by model
- ✅ Token usage summary
- ✅ Comprehensive test coverage

## Source

`src/cost.rs`