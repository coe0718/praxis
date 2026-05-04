# Usage

> Provider usage tracking, token/cost ledger, and budget enforcement

## Overview

The `usage` module tracks every LLM API call Praxis makes — recording input/output tokens, estimated costs, success/failure status, and per-phase breakdowns. It provides both the data structures for recording usage and a budget enforcement layer that can block requests when spending limits are exceeded.

Usage data flows through the system as follows:
1. Each LLM call produces a `ProviderAttempt` record.
2. Attempts are collected into `BackendOutput` by the backend module.
3. The Reflect phase persists attempts to SQLite via `ProviderUsageStore`.
4. Budget checks (if configured) can gate future requests.

The budget policy supports two modes:
- **Run mode** — For the normal agent loop (Orient/Decide/Act/Reflect). Higher limits.
- **Ask mode** — For one-shot operator questions via `praxis ask`. Lower limits.

## Architecture

### Key Types

- **`ProviderAttempt`** — A single LLM API call record: phase, provider, model, success, input/output tokens, estimated cost (microdollars), and optional error.
- **`ProviderUsageSummary`** — Aggregated usage for a session: attempt count, failure count, last provider, total tokens, total cost.
- **`TokenLedgerSummary`** — Per-session token and cost totals.
- **`PhaseTokenUsage`** — Token and cost breakdown by phase and provider.
- **`TokenSummaryAllTime`** — All-time totals: tokens, cost, session count.
- **`SessionTokenUsage`** — Per-session token and cost with day number.
- **`ProviderTokenSummary`** — Per-provider aggregate tokens and cost.

### Budget Enforcement

- **`UsageBudgetPolicy`** — Top-level policy with separate rules for `run` and `ask` modes.
- **`UsageBudgetRule`** — Per-mode limits: `max_attempts`, `max_tokens`, `max_cost_usd`.
- **`UsageBudgetMode`** — `Run` or `Ask`.
- **`UsageBudgetDecision`** — Result of a budget check: `blocked` flag and human-readable summary.

### Default Budget Limits

| Mode | Max Attempts | Max Tokens | Max Cost (USD) |
|---|---|---|---|
| Run | 6 | 3,000 | $0.25 |
| Ask | 1 | 600 | $0.05 |

## Public API

```rust
// Budget policy
UsageBudgetPolicy::load_or_default(path) -> Result<Self>
UsageBudgetPolicy::save_if_missing(path) -> Result<()>
UsageBudgetPolicy::rule(mode) -> &UsageBudgetRule
policy.validate() -> Result<()>

// Budget checks
rule.check_attempts(attempts, mode) -> UsageBudgetDecision
rule.check_estimate(estimated_tokens, mode) -> UsageBudgetDecision

// Token estimation
estimate_tokens(input: &str) -> i64

// Attempt helpers
ProviderAttempt::tokens_used(&self) -> i64  // input_tokens + output_tokens
```

## Configuration

### `usage_budget.toml` (optional)

```toml
[run]
max_attempts = 6
max_tokens = 3000
max_cost_usd = 0.25

[ask]
max_attempts = 1
max_tokens = 600
max_cost_usd = 0.05
```

If this file doesn't exist, default limits are used.

### Per-Route Cost Configuration

Set in `providers.toml`:

```toml
[[providers]]
provider = "openai"
model = "gpt-4o"
input_cost_per_million_usd = 2.50
output_cost_per_million_usd = 10.00
```

Costs are estimated as: `(tokens / 1,000,000) × price_per_million × 1,000,000` → microdollars.

## Usage

### CLI

```bash
# View usage summary
praxis status  # includes token totals and cost in the report

# View per-session usage
praxis usage sessions

# View per-provider usage
praxis usage providers

# View all-time totals
praxis usage total
```

### Programmatic

Budget checks are called before LLM requests in the runtime:

```rust
let policy = UsageBudgetPolicy::load_or_default(&paths.usage_budget_file)?;
let decision = policy.rule(UsageBudgetMode::Run).check_attempts(&attempts, UsageBudgetMode::Run);
if decision.blocked {
    // Skip this request
}
```

## Data Files

| File | Purpose |
|---|---|
| `data_dir/usage_budget.toml` | Budget policy (optional, defaults used if absent) |
| SQLite database | `provider_attempts` table with per-call records |

## Dependencies

- `storage` — `ProviderUsageStore` for persisting and querying attempts

## Source

`src/usage/` (`mod.rs`, `budget.rs`)
