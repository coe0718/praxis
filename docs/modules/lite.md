# Lite Mode

> Reduced-feature mode for constrained hardware — disables expensive capabilities while preserving safety guarantees.

## Overview

Lite mode reduces Praxis's resource footprint for Raspberry Pi, low-power servers, or any environment where CPU, memory, or API budget is limited. When enabled via `[agent] lite = true` in `praxis.toml`, it drops the context ceiling to 60%, disables speculative execution, skips sub-agent reviewers, stops synthetic example generation, cancels daily learning runs, skips morning brief delivery, and reduces anatomy refresh frequency.

Lite mode is about staying functional on constrained hardware, **not** about cutting safety. Approval queues, loop guards, and file-mutation circuit breakers all remain active. Individual capabilities can be toggled independently for fine-grained control.

The module also provides **fast mode** — a more aggressive reduction toggled via the `/fast` Telegram command or `--fast` CLI flag. Fast mode disables all expensive capabilities at once.

Additionally, the module manages **model overrides** — a file-based mechanism for live model switching without editing configuration files.

## Architecture

### Key Types

| Type | Description |
|---|---|
| `LiteMode` | Configuration struct with `enabled` flag and per-capability disable flags |
| `LiteCapability` | Enum of capabilities that can be gated: `Speculative`, `SubagentReviewer`, `SyntheticExamples`, `Learning`, `Brief`, `SseStream`, `Deterministic`, `Curator` |

### Capability Gating

```rust
lite.skip_capability(LiteCapability::Learning) // → true when lite is enabled
```

When `enabled` is `false`, `skip_capability()` always returns `false` (nothing is skipped).

### Default Behavior (lite enabled)

| Capability | Disabled? |
|---|---|
| Speculative execution | ✅ |
| Sub-agent reviewers | ✅ |
| Synthetic example generation | ✅ |
| Daily learning runs | ✅ |
| Morning brief | ✅ |
| Dashboard SSE stream | ✅ |
| Autonomous curator | ✅ (by default) |
| Deterministic mode | ❌ (enabled by default) |
| Anatomy refresh | Extended to 48h (from default) |
| Context ceiling | Lowered to 60% |

### Fast Mode

`LiteMode::fast_all()` creates a configuration that disables everything:
- Context ceiling: 50%
- All capabilities disabled
- Anatomy refresh: 24h

Toggled via the `fast_mode` flag file in the data directory.

### Model Override

Three functions manage a live model override flag file:
- `set_model_override(data_dir, "anthropic/claude-3-5-sonnet-latest")` — set override
- `get_model_override(data_dir)` — read override
- `clear_model_override(data_dir)` — remove override

## Public API

### Loading Lite Mode

```rust
let lite = LiteMode::from_file(&paths.config_file)?;
```

Reads `[agent]` section from `praxis.toml`. Returns defaults when file is missing.

### Capability Check

```rust
if lite.skip_capability(LiteCapability::Learning) {
    // Skip daily learning run
}
```

### Fast Mode Toggle

```rust
let now_fast = LiteMode::toggle_fast(&data_dir)?; // Returns new state
let is_fast = LiteMode::is_fast_active(&data_dir);
```

### Model Override

```rust
LiteMode::set_model_override(&data_dir, "anthropic/claude-3-5-sonnet-latest")?;
if let Some(model) = LiteMode::get_model_override(&data_dir) {
    println!("Using override model: {model}");
}
LiteMode::clear_model_override(&data_dir)?;
```

## Configuration

### `praxis.toml` — `[agent]` section (lite fields)

| Field | Type | Default | Description |
|---|---|---|---|
| `lite` | `bool` | `false` | Master switch for lite mode |
| `lite_context_ceiling_pct` | `float` | `0.60` | Context ceiling when lite is on |
| `lite_disable_speculative` | `bool` | `true` | Disable speculative execution |
| `lite_disable_subagent_reviewers` | `bool` | `true` | Skip sub-agent reviewer calls |
| `lite_disable_synthetic_examples` | `bool` | `true` | Skip synthetic example generation |
| `lite_disable_learning` | `bool` | `true` | Skip daily learning runs |
| `lite_disable_brief` | `bool` | `true` | Skip morning brief delivery |
| `lite_anatomy_refresh_hours` | `int` | `48` | Hours between anatomy refreshes |
| `lite_disable_sse` | `bool` | `true` | Disable dashboard SSE stream |
| `lite_disable_curator` | `bool` | `false` | Disable autonomous curator |
| `lite_disable_deterministic` | `bool` | `false` | Disable deterministic mode |

### CLI / Telegram

```bash
praxis --fast run --once    # run in fast mode
```

```
/fast    # toggle fast mode via Telegram
```

## Data Files

| File | Purpose |
|---|---|
| `fast_mode` | Flag file — presence means fast mode is active |
| `model_override` | Flag file — contains a `provider/model` string for live model switching |

## Dependencies

- External crates: `serde`, `toml`

## Source

`src/lite.rs`
