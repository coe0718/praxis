# Providers

> Inference provider registry, route configuration, and cost estimation

## Overview

The `providers` module defines the data model for LLM inference providers — their routes, protocols, routing classes, cost parameters, and traffic weights. It is the configuration layer that the `backend` module consumes at runtime to select and dispatch to specific LLM APIs.

Praxis ships with 15 built-in provider routes covering major inference APIs (Anthropic, OpenAI, Ollama, GitHub Copilot, Moonshot/Kimi, MiniMax, GLM, Azure, Bedrock, NVIDIA, Vercel, GMI, Arcee, Step, and LM Studio). Additional providers can be added by setting `base_url` in `providers.toml` — any custom name with a base URL is treated as an OpenAI-compatible endpoint.

The module also handles environment-enriched discovery: if a standard API key (e.g., `ANTHROPIC_API_KEY`) is present but no route exists for that provider, a default route is automatically added. This allows zero-config operation for common setups.

## Architecture

### Key Types

- **`ProviderSettings`** — Top-level configuration: a list of `ProviderRoute` entries. Loaded from `providers.toml` with environment variable enrichment.
- **`ProviderRoute`** — A single provider route definition with fields:
  - `provider` — Provider name (e.g., `"claude"`, `"openai"`, or a custom name)
  - `model` — Default model identifier
  - `base_url` — Optional API endpoint override
  - `protocol` — Optional wire protocol override (`anthropic`, `openai-compat`, `ollama`)
  - `class` — Optional routing class (`fast`, `reliable`, `local`)
  - `input_cost_per_million_usd` / `output_cost_per_million_usd` — Optional cost parameters
  - `weight` — Optional static traffic weight (0.0–1.0)

- **`ProviderProtocol`** — Wire protocol enum: `Anthropic`, `OpenAiCompat` (default), `Ollama`. Inferred from provider name when not explicitly set.

- **`RouteClass`** — Routing class enum: `Fast` (cheap/low-latency), `Reliable` (capable/high-quality), `Local` (on-device).

### Protocol Resolution

Provider name → protocol mapping:
- `"claude"` → `Anthropic`
- `"ollama"` → `Ollama`
- Everything else → `OpenAiCompat`

### Cost Estimation

`ProviderRoute::estimated_cost_micros(input_tokens, output_tokens)` computes cost in microdollars using the configured per-million-token prices. Returns 0 if prices are not configured.

### Environment Enrichment

`ProviderSettings::env_enriched()` checks for:
- `ANTHROPIC_API_KEY` → adds default Claude route
- `OPENAI_API_KEY` (+ optional `OPENAI_BASE_URL`) → adds default OpenAI route
- `OLLAMA_HOST` → adds default Ollama route

Existing configured routes are never overwritten.

## Public API

```rust
// Loading and saving
ProviderSettings::load_or_default(path: &Path) -> Result<Self>
ProviderSettings::save_if_missing(path: &Path) -> Result<()>
ProviderSettings::validate() -> Result<()>

// Route lookup
settings.first_for(provider: &str) -> Option<ProviderRoute>
settings.first_for_class(class: &RouteClass) -> Option<ProviderRoute>

// Route properties
route.resolved_protocol() -> ProviderProtocol
route.validate() -> Result<()>
route.uses_oauth() -> bool
route.estimated_cost_micros(input_tokens, output_tokens) -> i64
```

## Configuration

### `providers.toml`

```toml
[[providers]]
provider = "claude"
model = "claude-3-5-sonnet-latest"

[[providers]]
provider = "openai"
model = "gpt-4o-mini"

[[providers]]
provider = "ollama"
model = "llama3.2"
base_url = "http://127.0.0.1:11434"

[[providers]]
provider = "custom-llm"
model = "my-model"
base_url = "https://api.example.com/v1"
protocol = "openai-compat"
class = "fast"
input_cost_per_million_usd = 0.15
output_cost_per_million_usd = 0.60
weight = 0.5
```

### Well-Known Providers

The following provider names are recognized without requiring `base_url`:

`claude`, `openai`, `ollama`, `copilot`, `kimi`, `minimax`, `glm`

All other names require `base_url` to be set (they are dispatched as OpenAI-compatible endpoints).

## Usage

### CLI

```bash
# View current provider configuration
praxis model list

# Switch the active model
praxis model gpt-4o
```

### Default Providers

The built-in default configuration includes routes for:

| Provider | Default Model | Base URL |
|---|---|---|
| claude | claude-3-5-sonnet-latest | (Anthropic default) |
| openai | gpt-4o-mini | (OpenAI default) |
| ollama | llama3.2 | http://127.0.0.1:11434 |
| copilot | gpt-4o | https://api.githubcopilot.com |
| kimi | moonshot-v1-8k | https://api.moonshot.cn |
| minimax | abab6.5s-chat | https://api.minimax.chat |
| glm | glm-4 | https://open.bigmodel.cn/api/paas/v4 |
| azure | gpt-4o | (template) |
| bedrock | anthropic.claude-3-sonnet | (template) |
| nvidia | llama-3.1-nemotron-70b-instruct | https://integrate.api.nvidia.com |
| vercel | ai-default | https://api.vercel.ai |
| gmi | gmi-default | https://api.gmi.cloud |
| arcee | arcee-default | https://api.arcee.ai |
| step | step-default | https://api.step.ai |
| lmstudio | lmstudio-default | http://localhost:1234/v1 |

## Data Files

| File | Purpose |
|---|---|
| `data_dir/providers.toml` | Provider route configuration |

## Dependencies

- `serde` — TOML/JSON serialization
- `toml` — Configuration file parsing

## Source

`src/providers/mod.rs`
