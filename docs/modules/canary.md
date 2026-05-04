# Canary

> Model canary health checks, freeze/rollback automation, and dynamic traffic weights

## Overview

The `canary` module implements a continuous health-check system for LLM providers. It sends a simple probe prompt ("Reply with exactly: PraxisCanaryReady") to each configured provider, runs the local eval suite against the response, and records the result in a persistent ledger.

Beyond simple pass/fail tracking, the canary system includes **automation** that dynamically adjusts traffic routing:
- Providers that fail after a passing streak are **immediately frozen** (rolled back).
- Providers with sustained failures lose traffic weight gradually (25% per run).
- Providers that recover with 3+ consecutive passes are **promoted** back to full traffic.

This creates a self-healing provider infrastructure where degraded models are automatically deprioritized and restored without operator intervention.

## Architecture

### Key Types

- **`ModelCanaryLedger`** — Persistent record of the latest canary check per provider/model pair. Stored as `model_canary.json`.
- **`ModelCanaryRecord`** — A single check result: provider, model, status (Passed/Failed), timestamp, summary, eval failure count, and consecutive pass count.
- **`CanaryStatus`** — `Passed` or `Failed`.
- **`CanaryFreezeState`** — Set of currently frozen `"provider/model"` routes. Stored as `canary_freeze.json`.
- **`RouteWeightStore`** — Dynamic traffic weights (0.0–1.0) per route. Stored as `canary_weights.json`. Routes not in the file default to 1.0 (full traffic).
- **`CanaryAutomation`** — Result of applying automation: lists of promoted, rolled-back, weight-reduced, and weight-increased routes.

### Constants

| Constant | Value | Purpose |
|---|---|---|
| `PROMOTION_THRESHOLD` | 3 | Consecutive passes needed to unfreeze a route |
| `WEIGHT_STEP_DOWN` | 0.25 | Traffic reduction per failure |
| `WEIGHT_STEP_UP` | 0.125 | Traffic increase per pass during recovery |

### Automation Logic

```
For each record in the latest ledger:
  if Passed:
    if consecutive_passes >= 3 AND frozen → unfreeze, restore weight to 1.0
    else if not frozen → increase weight by 0.125 (capped at 1.0)
  if Failed:
    if was previously passing → immediate freeze (rollback), weight = 0.0
    else → reduce weight by 0.25; auto-freeze at 0.0
```

## Public API

```rust
// Running canaries
run_canaries(config, paths, provider: Option<&str>) -> Result<Vec<ModelCanaryRecord>>

// Automation
apply_automation(ledger, freeze_path, weights_path, prev_ledger) -> Result<CanaryAutomation>

// Ledger management
ModelCanaryLedger::load_or_default(path) -> Result<Self>
ledger.save(path) -> Result<()>
ledger.replace(record)                    // Upsert with consecutive pass tracking
ledger.passed(provider, model) -> bool

// Freeze state
CanaryFreezeState::load_or_default(path) -> Result<Self>
freeze.is_frozen(provider, model) -> bool
freeze.freeze(provider, model)
freeze.unfreeze(provider, model)

// Route weights
RouteWeightStore::load_or_default(path) -> Result<Self>
weights.get(provider, model) -> f64       // Default 1.0
weights.set(provider, model, weight)      // Clamped to [0.0, 1.0]
```

## Configuration

### `praxis.toml`

| Field | Description | Default |
|---|---|---|
| `agent.freeze_on_model_regression` | Enable canary gating in the backend | `false` |
| `agent.backend` | When set to `"router"`, canaries run on all unique providers | — |

### Canary Probe

The probe prompt is fixed: `"Reply with exactly: PraxisCanaryReady"`. The provider must return this string and pass the `LocalEvalSuite` (triggered by `EvalTrigger::Canary`).

## Usage

### CLI

```bash
# Run canaries for all configured providers
praxis canary run

# Run canary for a specific provider
praxis canary run --provider claude

# View canary status
praxis canary status

# Manually unfreeze a route
praxis canary unfreeze claude claude-3-5-sonnet-latest
```

### Automatic

- The backend's `CanaryGate` checks the ledger on every request and filters out frozen routes.
- The `RouterBackend` uses `RouteWeightStore` to weight traffic toward healthier providers.
- Canary automation (`apply_automation`) runs after every `run_canaries()` call.

## Data Files

| File | Purpose |
|---|---|
| `data_dir/model_canary.json` | Latest canary check results per provider/model |
| `data_dir/canary_freeze.json` | Set of currently frozen routes |
| `data_dir/route_weights.json` | Dynamic traffic weights from canary automation |

## Dependencies

- `backend` — `AgentBackend`, `ConfiguredBackend`
- `config` — `AppConfig`
- `providers` — `ProviderSettings`
- `quality` — `LocalEvalSuite`, `EvalTrigger`
- `storage` — `EvalStatus`, `EvalSeverity`

## Source

`src/canary.rs`
