# Codex CLI Model + Auth Compatibility

Reference for error messages and model restrictions encountered with Codex CLI 0.118.0.

## Auth Modes

Two modes live in `~/.codex/auth.json`:

| `auth_mode` | Source | Model Flexibility |
|---|---|---|
| `"chatgpt"` | OAuth via ChatGPT web login | Restricted to ChatGPT account tier |
| `"api_key"` | `OPENAI_API_KEY` env var or config | Full OpenAI API model access |

## Known Error Messages

### "requires a newer version of Codex"

```
The 'gpt-5.5' model requires a newer version of Codex. Please upgrade to the
latest app or CLI and try again.
```

**Cause**: `config.toml` has `model = "gpt-5.5"` but the installed CLI (e.g., 0.118.0) doesn't support it yet. gpt-5.x models require a newer Codex release.

**Fix**: Either upgrade CLI (`npm update -g @openai/codex`) or remove the model line from config.toml to use the default.

### "not supported when using Codex with a ChatGPT account"

```
The 'o4-mini' model is not supported when using Codex with a ChatGPT account.

The 'gpt-4o' model is not supported when using Codex with a ChatGPT account.
```

**Cause**: The model is either not available on ChatGPT tiers at all, or the specific ChatGPT account (e.g., free/plus/pro) doesn't include it.

**Fix**: Remove the `model` line from `~/.codex/config.toml` — let the CLI pick its default. The default is guaranteed compatible with the current auth mode and CLI version.

### "stdin is not a terminal"

```
Error: stdin is not a terminal
```

**Cause**: Running `codex` (no subcommand) or `codex models` (which requires TTY) inside a non-interactive Hermes terminal or subprocess.

**Fix**: Don't use commands that require interactive TTY. Use `ps aux | grep codex` to see what model is actually running instead.

## What Works With ChatGPT Auth

| Model | CLI 0.118.0 + ChatGPT Auth | Notes |
|---|---|---|
| Default (no model set) | ✅ Works | Falls back to account default |
| gpt-4.1 | ✅ Likely | Available on ChatGPT Plus |
| o3 | ✅ Likely | Available on ChatGPT Plus |
| gpt-4o | ❌ | "not supported when using Codex with a ChatGPT account" |
| o4-mini | ❌ | "not supported when using Codex with a ChatGPT account" |
| gpt-5.5 | ❌ | Requires newer CLI version |

## How to Check Current Model

```bash
# Can't use `codex models` (needs TTY).
# Check config for explicit setting:
cat ~/.codex/config.toml | grep model

# Check what's actually running:
ps aux | grep codex | grep -v grep | head -1
# The model name may appear in the process args.
```
