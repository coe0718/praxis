# OpenRouter Model 400 Errors — Debugging Guide

Model slug mismatches on OpenRouter produce HTTP 400 with a misleading error message. Use this process to diagnose.

## Symptoms

- Agent returns 400 errors on OpenRouter
- Error log entry: `openai.BadRequestError: Error code: 400 - {'error': {'message': '<something> is not a valid model ID', 'code': 400}}`
- Gateway starts fine but fails on every API call

## Root Cause

Usually an incorrect model slug in the agent's `config.yaml`. OpenRouter model IDs are case-sensitive and slug-specific. For example:

| Wrong | Correct |
|-------|---------|
| `minimaxai/minimax-m2.5:free` | `minimax/minimax-m2.5:free` |
| `deepseek-ai/deepseek-v4-pro:free` | `deepseek/deepseek-v4-pro:free` |

## Debug Steps

### 1. Check the error log

```bash
tail -20 /home/coemedia/.hermes/profiles/<profile>/logs/errors.log
```

The exact model ID OpenRouter rejected is in the error message. Compare it against their model catalog.

### 2. Verify the configured slug

```bash
grep -A2 "default:" /home/coemedia/.hermes/profiles/<profile>/config.yaml | head -4
```

### 3. Look up the correct slug on OpenRouter

Search `openrouter.ai/models` for the model, or use:

```bash
curl -s "https://openrouter.ai/api/v1/models" | python3 -c "import sys,json; data=json.load(sys.stdin); [print(m['id']) for m in data['data'] if 'minimax' in m['id'].lower()]"
```

### 4. Check context length

Don't assume free tier caps context — verify on OpenRouter's model page:

```bash
curl -s "https://openrouter.ai/api/v1/models" | python3 -c "
import sys,json; data=json.load(sys.stdin)
for m in data['data']:
    if 'minimax/m2.5' in m['id']:
        print(f\"{m['id']}: {m.get('context_length', 'unknown')} context\")
"
```

Free models often retain full context (MiniMax M2.5 free = 196,608).

### 5. Check gateway status

```bash
ps aux | grep "profile <name>" | grep gateway
```

Gateway restarts don't log the config slug — the error only surfaces on the first API call.

### 6. Common pitfalls

- **Trailing `:free`** is required for free OpenRouter models (`minimax/minimax-m2.5:free`, NOT `minimax/minimax-m2.5`)
- **Scout had terminal.cwd: messages** which caused all file tools to fail with FileNotFound. When debugging an agent, check `cwd` in its terminal config too — a 400 error might mask other config issues
- **The old `minimaxai/` prefix** was used in early OpenRouter releases but was deprecated — the correct prefix is `minimax/`
- **Gateway restart** is required after config changes: `kill <PID>` and the gateway auto-restarts via systemd or supervisor
- **Errors.log** lives at `~/.hermes/profiles/<profile>/logs/errors.log` — gateway.log may not show the full error context
