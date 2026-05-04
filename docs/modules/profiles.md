# Profiles

> Named model profiles in `profiles.toml` — swap backend, context limits, and tool budgets with a single config value.

## Overview

The profiles module manages named execution profiles that adjust agent behavior without editing `praxis.toml` directly. A profile can override the backend provider, context ceiling, context window token count, maximum tool calls per session, and sub-agent spawning policy. The active profile is selected by the `agent.profile` field in `praxis.toml`.

Profiles are loaded from `profiles.toml` and applied on top of `AppConfig` before each session. This lets operators define presets like "quality" (full power), "budget" (cost-optimized routing), "offline" (local-only Ollama), "lite" (constrained for Raspberry Pi), and "deterministic" (stub backend for testing).

## Architecture

### Key Types

| Type | Description |
|---|---|
| `ProfileSettings` | Container for a list of `ExecutionProfile` entries |
| `ExecutionProfile` | A named profile with optional overrides for backend, context ceiling, token budget, tool call cap, and sub-agent policy |

### Built-in Profiles

| Profile | Backend Override | Context Ceiling | Window Tokens | Max Tool Calls | Sub-agents |
|---|---|---|---|---|---|
| `quality` | (none) | (none) | (none) | (none) | (none) |
| `budget` | `router` | 65% | (none) | (none) | (none) |
| `offline` | `ollama` | 55% | (none) | (none) | (none) |
| `deterministic` | `stub` | 30% | (none) | (none) | (none) |
| `lite` | (none) | 50% | 6,000 | 5 | disabled |

## Public API

### Loading

```rust
let settings = ProfileSettings::load_or_default(&paths.profiles_file)?;
```

Returns built-in defaults when the file doesn't exist.

### Applying

```rust
let adjusted_config = settings.apply(&config)?;
```

Looks up the profile matching `config.agent.profile`, applies overrides, and re-validates the resulting config.

### Saving

```rust
settings.save_if_missing(&paths.profiles_file)?;
```

Writes defaults only if the file doesn't already exist (preserves customizations).

### Validation

```rust
settings.validate()?;
```

Checks: at least one profile, no empty names, no duplicates, valid backend overrides, valid context ceilings.

## Configuration

### `profiles.toml` Format

```toml
[[profiles]]
name = "quality"

[[profiles]]
name = "budget"
backend_override = "router"
local_first_fallback = true
context_ceiling_pct = 0.65

[[profiles]]
name = "offline"
backend_override = "ollama"
local_first_fallback = true
context_ceiling_pct = 0.55

[[profiles]]
name = "lite"
context_ceiling_pct = 0.50
context_window_tokens = 6000
max_session_tool_calls = 5
disable_sub_agents = true

[[profiles]]
name = "deterministic"
backend_override = "stub"
local_first_fallback = false
context_ceiling_pct = 0.30
```

### `praxis.toml` — selecting a profile

```toml
[agent]
profile = "budget"  # or "quality", "offline", "lite", "deterministic"
```

### CLI

```bash
praxis profile list       # show available profiles
praxis profile set budget # switch active profile
```

## Data Files

| File | Purpose |
|---|---|
| `profiles.toml` | Named execution profiles. Auto-created with defaults on first run. |

## Dependencies

- **config** — `AppConfig` as the base configuration that profiles modify

## Source

`src/profiles.rs`
