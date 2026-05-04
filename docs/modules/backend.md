# Backend

> LLM backend abstraction, provider routing, and multi-provider failover

## Overview

The `backend` module provides the core interface between Praxis and its LLM providers. It defines the `AgentBackend` trait — the contract that the runtime's Orient/Decide/Act/Reflect loop uses to interact with language models — and implements it through a layered routing system that supports single-provider, multi-provider router, and stub configurations.

The module handles:
- **Provider selection** — Single provider, router (multi-provider with class-based routing), or stub (no-op for testing).
- **Failover** — When a provider fails, the system automatically tries the next route in the ordered list.
- **Health tracking** — An in-memory health tracker marks providers unhealthy after 3 consecutive failures and auto-recovers on success.
- **Canary gating** — If `freeze_on_model_regression` is enabled, providers that failed their last canary check are excluded from routing.
- **Model override** — A thread-safe mechanism for live model switching without config changes.
- **Prompt construction** — Phase-specific system prompts for ask, decide, and act phases.
- **Streaming** — An async streaming backend for dashboard/SSE use cases.

## Architecture

### Core Trait

```rust
pub trait AgentBackend {
    fn name(&self) -> &'static str;
    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput>;
    fn plan_action(&self, goal, task, context) -> Result<BackendOutput>;
    fn finalize_action(&self, planned_summary, goal, task, context) -> Result<BackendOutput>;
}
```

### Backend Variants

- **`ConfiguredBackend::Stub`** — Returns canned responses. Used for testing and dry-run mode.
- **`ConfiguredBackend::Single(Box<SingleBackend>)`** — Routes to one primary provider with optional local-first fallback to Ollama.
- **`ConfiguredBackend::Router(RouterBackend)`** — Multi-provider routing with class-based selection (`Fast`, `Reliable`, `Local`), weight-based ordering, and dynamic canary weights.

### Routing Logic (Router Mode)

1. Routes are bucketed by class match: matched → unclassed → rest.
2. Within each bucket, routes are sorted by effective weight (static × dynamic from canary automation).
3. Local-first flag moves Ollama routes to the front for `ask`/`act` phases.
4. Canary gate filters out frozen routes.
5. Health tracker filters out unhealthy providers (with fallback to trying all if none are healthy).
6. Routes are tried in order until one succeeds.

### Key Types

| Type | Purpose |
|---|---|
| `BackendOutput` | Response with summary text and provider attempt records |
| `ProviderRequest` | Outbound request: phase, system prompt, input content, max tokens |
| `ProviderResponse` | Inbound response: summary text + one provider attempt |
| `ContentBlock` | Multi-modal content (text or image URL) |
| `InputContent` | Either plain text or multi-modal blocks |
| `SingleBackend` | One primary route + optional Ollama fallback |
| `RouterBackend` | Multiple routes with class-based, weight-aware selection |
| `ModelOverride` | Thread-safe in-memory model override cell |

### Submodules

| Submodule | Purpose |
|---|---|
| `configured` | `ConfiguredBackend` enum with routing and failover logic |
| `openai` | OpenAI-compatible provider adapter |
| `claude` | Anthropic Claude provider adapter |
| `ollama` | Ollama local provider adapter |
| `streaming` | Async SSE streaming for dashboard sessions |
| `transport` | Pluggable transport trait and registry |
| `provider_routes` | Route resolution, model pin/override, credential validation |
| `prompts` | Phase-specific system prompt construction |
| `model_override` | Thread-safe model override cell (global singleton) |
| `attempts` | Provider attempt record helpers |
| `credential_pool` | API key pool initialization |
| `gating` | Canary-based route filtering (`CanaryGate`) |
| `health` | Provider health tracking with auto-failover |

## Public API

```rust
// Backend construction
ConfiguredBackend::from_runtime(config, paths) -> Result<Self>
ConfiguredBackend::validate_environment(config, paths) -> Result<()>

// AgentBackend trait methods (called by the runtime loop)
backend.answer_prompt(prompt) -> Result<BackendOutput>
backend.plan_action(goal, task, context) -> Result<BackendOutput>
backend.finalize_action(planned_summary, goal, task, context) -> Result<BackendOutput>

// Raw request (bypasses prompt templates)
backend.execute_raw_request(request) -> Result<BackendOutput>

// Model override
ModelOverride::new() -> Self
model_override.set(model)
model_override.clear()
model_override.get() -> Option<String>
global_model_override() -> &'static ModelOverride

// Streaming
StreamingBackend::new() -> Result<Self>
backend.stream_completion(route, request) -> impl Stream<Item = Result<StreamEvent>>
```

## Configuration

### `praxis.toml` Fields

| Field | Description | Default |
|---|---|---|
| `agent.backend` | Provider mode: `"stub"`, `"router"`, or a provider name (`"claude"`, `"openai"`, `"ollama"`, etc.) | `"claude"` |
| `agent.model_pin` | Pin a specific model name for the configured backend | `null` |
| `agent.freeze_on_model_regression` | Enable canary gating to freeze failing providers | `false` |
| `agent.local_first_fallback` | Prepend Ollama routes for ask/act phases | `false` |
| `agent.prompt_caching` | Enable prompt caching hints (Anthropic) | `false` |

### Environment Variables

| Variable | Provider | Description |
|---|---|---|
| `ANTHROPIC_API_KEY` | Claude | Required for Anthropic provider |
| `OPENAI_API_KEY` | OpenAI | Required for OpenAI provider |
| `<UPPER_PROVIDER>_API_KEY` | Custom | Per-provider API key |
| `PRAXIS_CLAUDE_STUB_RESPONSE` | Claude | Stub response for testing |
| `PRAXIS_OPENAI_STUB_RESPONSE` | OpenAI | Stub response for testing |
| `OLLAMA_HOST` | Ollama | Override Ollama base URL |
| `OPENAI_BASE_URL` | OpenAI | Override OpenAI base URL |
| `PRAXIS_PROVIDER_HEALTH_FILE` | All | Path for health state persistence |

### Model Override Hierarchy

1. `ModelOverride` in-memory (set via `/model` REPL command or API) — highest priority
2. `model_override` file in data directory (set via `praxis model <name>`)
3. `agent.model_pin` in `praxis.toml`
4. Default model from provider settings

## Usage

### CLI

```bash
# Switch model live
praxis model claude-3-5-sonnet-latest

# Check backend status
praxis status
```

## Data Files

| File | Purpose |
|---|---|
| `data_dir/providers.toml` | Provider route configuration |
| `data_dir/model_override` | File-based model override |
| `data_dir/canary_freeze.json` | Frozen routes from canary automation |
| `data_dir/route_weights.json` | Dynamic route weights from canary automation |
| `data_dir/model_canary.json` | Canary check results |

## Dependencies

- `providers` — `ProviderRoute`, `ProviderSettings`, `ProviderProtocol`, `RouteClass`
- `config` — `AppConfig`
- `canary` — `RouteWeightStore`, `ModelCanaryLedger`
- `paths` — `PraxisPaths`
- `usage` — `ProviderAttempt`
- `identity` — `Goal`

## Source

`src/backend/`
