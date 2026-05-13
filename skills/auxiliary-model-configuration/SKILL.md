---
name: auxiliary-model-configuration
category: devops
description: Identify and configure auxiliary models (vision, web extract, compression) across Hermes multi-agent system to control costs and use appropriate free models.
author: Tuck
created: 2026-04-19
tags: auxiliary, models, cost, openrouter, multi-agent
---

## Problem

Hermes auxiliary tasks (vision, web extraction, context compression) use hardcoded default models that may not match the user's configuration. When auxiliary models are unset (provider: auto, model: empty), the system falls back to google/gemini-3-flash-preview — which can incur unexpected charges.

## Where Auxiliary Models Are Used

The auxiliary client (hermes-agent/agent/auxiliary_client.py) handles:
- Vision — image analysis when users send photos
- Web extract — fetching and parsing web pages
- Compression — context summarization when conversations get long

Hardcoded defaults:
```python
_OPENROUTER_MODEL = "google/gemini-3-flash-preview"
_NOUS_MODEL = "google/gemini-3-flash-preview"
```

## Configuration Locations

### Main gateway config
config.yaml auxiliary section controls vision, web_extract, and compression for the main Tuck gateway.

### Agent profile configs
Each agent profile has its own auxiliary section that overrides the default.

## How to Audit Current Configuration

```bash
grep -A10 "auxiliary" config.yaml
grep -r "auxiliary\|vision\|web_extract\|summary_model" profiles/*/config.yaml
```

## How to Switch Auxiliary Models

### Main config update
Edit config.yaml auxiliary section:
```yaml
auxiliary:
  vision:
    provider: openrouter
    model: nvidia/nemotron-nano-12b-v2-vl:free
  web_extract:
    provider: openrouter
    model: z-ai/glm-4.5-air:free
  compression:
    provider: auto
    model: minimax/minimax-m2.5:free
```

### Agent profile updates
Update each agent config individually - the main config does not cascade.

### Restart all services
All 8 agents run as systemd services:
```bash
systemctl --user restart hermes-gateway-{drey,vex,scout,echo,herald,sable,locke,maren}
```
Council agents (sable, locke, maren) are systemd services too, not direct processes.

## Recommended Free Models (OpenRouter)

| Task | Model | Context | Notes |
|------|-------|---------|-------|
| Vision | nvidia/nemotron-nano-12b-v2-vl:free | 128K | Multimodal, image capable |
| Web extract | z-ai/glm-4.5-air:free | 131K | Text, reasoning |
| Compression | minimax/minimax-m2.5:free | 197K | Largest free context window |

Avoid: google/gemma models — frequently return 500 errors on OpenRouter.

## Pitfalls

1. Compression model context too small — if the compression model context is smaller than main_model_context x compression.threshold, summarization will fail. Error: "Compression model context is X tokens, but the main model's compression threshold is Y tokens."
   Fix options:
   - Use a larger compression model (e.g., minimax/minimax-m2.5:free at 197K)
   - Lower compression.threshold in config.yaml (e.g., 0.12) so compression triggers earlier

2. Each profile needs its own config — main config does not cascade to agent profiles.

3. Agent summary_model is separate from auxiliary.compression.model — both need updating.

4. Services must be restarted for config changes to take effect.

## Related Skills

- rate-limit-mitigation: Caching and middleware for API rate limits
- system-health-monitor: System monitoring and alerting
