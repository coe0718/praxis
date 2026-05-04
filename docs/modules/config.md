# Config

> Configuration model, validation, security overrides, and hot-reload watcher for `praxis.toml`.

## Overview

The config module defines `AppConfig` — the typed representation of `praxis.toml`, Praxis's main configuration file. It covers instance identity, runtime behavior, database settings, security policy, agent tuning, context budgets, feature flags, and ephemeral prompts. Every field has sensible defaults and is validated on load to catch misconfiguration early.

The module also provides `SecurityOverrides` (a separate `security.toml` for sensitive overrides that shouldn't be committed to version control) and `ConfigWatcher` (a background file watcher that enables hot-reload — changes to `praxis.toml` take effect on the next daemon cycle without a restart).

## Architecture

### Key Types

| Type | Description |
|---|---|
| `AppConfig` | Root config struct, loaded from `praxis.toml`. Contains instance, runtime, database, security, agent, context, features, and ephemeral_prompts sections. |
| `InstanceConfig` | Instance name, timezone, and data directory |
| `RuntimeConfig` | Quiet hours, state file path, backup settings |
| `DatabaseConfig` | SQLite database path |
| `SecurityConfig` | Security level (1–4), tool cooldowns, secret redaction, mention requirements, user allowlists, hardline blocklists |
| `AgentConfig` | Backend selection, context ceiling, profile, model pin, prompt caching, fallback providers, role, spawn depth, inactivity timeout |
| `ContextConfig` | Context window token budget and per-source allocation |
| `ContextSourceConfig` | A named context source with priority and max percentage |
| `FeatureFlags` | Runtime feature toggles (all default off) |
| `FeatureFlag` | Enum for typed flag checking |
| `SecurityOverrides` | Sensitive overrides from `security.toml` |
| `ConfigWatcher` | Background filesystem watcher for hot-reload |

### Config Sections in `praxis.toml`

```toml
[instance]
name = "Axonix"
timezone = "America/New_York"
data_dir = "/home/user/.local/share/praxis"

[runtime]
quiet_hours_start = "23:00"
quiet_hours_end = "07:00"
state_file = "session_state.json"
daily_backup_snapshots = false
snapshot_retention_days = 7
backup_before_update = true

[database]
path = "praxis.db"

[security]
level = 2
redact_secrets = false
require_mention = false
allowed_users = []
hardline_blocklist = []

[agent]
backend = "claude"
context_ceiling_pct = 0.80
profile = "quality"
model_pin = "claude-3-5-sonnet-latest"
prompt_caching = true
prompt_cache_ttl_secs = 300
extended_prompt_cache = false
fallback_providers = ["openai", "ollama"]
role = "worker"
max_spawn_depth = 0
inactivity_timeout_secs = 600
disable_sub_agents = false
max_session_tool_calls = 10

[context]
window_tokens = 12000

[[context.budget]]
source = "soul"
priority = 1
max_pct = 0.03

[features]
credential_pooling = false
cron_tool = false
speculative_execution = false
delegation = false
mcp_dispatch = false

[ephemeral_prompts]
"chat:123456" = "Focus on code review today"
```

### Feature Flags

| Flag | Description |
|---|---|
| `credential_pooling` | Multi-key rotation per provider |
| `cron_tool` | Agent-callable scheduled jobs |
| `speculative_execution` | Branching Act phase with conservative alternative |
| `delegation` | Agent-to-agent delegation links |
| `mcp_dispatch` | Model Context Protocol tool dispatch |

### Context Budget Defaults

| Source | Priority | Max % |
|---|---|---|
| soul | 1 | 3% |
| identity | 2 | 5% |
| operator_model | 3 | 3% |
| agents | 4 | 4% |
| active_goals | 5 | 9% |
| do_not_repeat | 6 | 7% |
| known_bugs | 7 | 6% |
| memory_hot | 8 | 17% |
| memory_cold | 9 | 11% |
| predictions | 10 | 4% |
| patterns | 11 | 4% |
| journal | 12 | 5% |
| tools | 13 | 4% |
| task | 14 | 18% |

## Public API

### Loading and Saving

```rust
let config = AppConfig::load(&paths.config_file)?;
config.save(&paths.config_file)?;
let config = AppConfig::default_for_data_dir(data_dir);
```

### Validation

```rust
config.validate()?;  // Called automatically by load() and save()
```

Validates: non-empty name, valid timezone, valid quiet-hours times, security level 1–4, valid backend name, context ceiling 0–1, unique budget priorities, etc.

### Mutation

```rust
config.set_name("MyAgent");
config.set_timezone("Europe/London")?;
let config = config.with_overridden_data_dir(new_dir);
```

### Security Overrides

```rust
let overrides = SecurityOverrides::load_or_default(&paths.security_file)?;
// overrides.level — optional security level override (1-4)
// overrides.note  — free-text operator memo
```

### Config Watcher (daemon)

```rust
let watcher = ConfigWatcher::spawn(paths.config_file.clone())?;
// Later, in the daemon loop:
if watcher.take_dirty() {
    let config = AppConfig::load(&paths.config_file)?;
    // Use updated config...
}
```

## Configuration

### File Locations

| File | Purpose |
|---|---|
| `praxis.toml` | Main config — committable to version control |
| `security.toml` | Sensitive overrides — add to `.gitignore` |

### Validation Rules

- `instance.name` must not be empty
- `instance.timezone` must be a valid IANA timezone
- `runtime.quiet_hours_start/end` must be valid `HH:MM` format
- `security.level` must be 1–4
- `agent.backend` must be one of: `stub`, `claude`, `openai`, `ollama`, `router`
- `agent.context_ceiling_pct` must be > 0.0 and ≤ 1.0
- `agent.prompt_cache_ttl_secs` must be > 0
- `agent.extended_prompt_cache` requires `agent.prompt_caching = true`
- `agent.max_spawn_depth` must be > 0 when `agent.role = "orchestrator"`
- `agent.inactivity_timeout_secs` must be ≥ 60 if set
- `context.budget` priorities must be unique
- `context.budget` max_pct must be > 0.0 and ≤ 1.0

## Dependencies

- **time** — `parse_timezone()`, `parse_clock_time()` for validation
- External crates: `toml`, `serde`, `notify` (for ConfigWatcher)

## Source

`src/config/` — `mod.rs`, `model.rs`, `security.rs`, `validation.rs`, `watcher.rs`, `tests.rs`
