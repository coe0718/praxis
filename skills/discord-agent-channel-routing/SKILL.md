---
name: discord-agent-channel-routing
category: devops
description: Fix Discord multi-agent channel routing — slash commands bypass channel filters, sync failures, and bot isolation issues.
---

# Discord Multi-Agent Channel Routing

Troubleshooting routing issues when multiple Hermes agents (each with their own Discord bot) share a guild.

## Known Issues

### 1. Slash Commands ~~Bypass~~ Respect Channel Filters (FIXED)
**Status:** Fixed in `discord.py` `_run_simple_slash()` (~line 1710).

The channel filter is now checked before dispatching to `handle_message()`:
```python
channel_id = str(interaction.channel_id)
allowed_raw = os.getenv("DISCORD_ALLOWED_CHANNELS", "")
if allowed_raw:
    allowed = {ch.strip() for ch in allowed_raw.split(",") if ch.strip()}
    if channel_id not in allowed:
        await interaction.response.send_message(
            "This bot isn't configured for this channel.", ephemeral=True
        )
        return
```

**If still not working:** Restart the gateway — env var changes require a service restart.

### 2. Slash Command Sync Failure — "Command exceeds maximum size (8000)" ✅ FIXED
**Status:** Fixed 04/17 by removing 24 duplicate manual commands from `_register_slash_commands`.

**Root Cause:** Manual slash command definitions duplicated commands already auto-registered from `COMMAND_REGISTRY`. The combined payload (60+ commands) exceeded Discord's 8000-byte limit.

**Fix applied:** Only `new`, `reset`, and `thread` remain as manual commands (need custom behavior). Everything else is auto-registered. ~3500 bytes payload now.

**For new setups:** If sync fails again, check `COMMAND_REGISTRY` size — each agent auto-registers all gateway-available commands from the shared registry.

### 3. Multiple Bots Registering Same Slash Commands
**Symptom:** Typing `/reset` shows multiple options (one per bot). User accidentally picks the wrong one.

**Context:** Each bot registers global slash commands. Discord shows all registered commands from all bots in the guild.

**Mitigation:** Each agent's `DISCORD_ALLOWED_CHANNELS` should prevent wrong-bot responses for regular messages. For slash commands, see issue #1 above.

## Configuration Checklist

Each agent's `.env` should have:
```bash
# Unique bot token per agent
DISCORD_BOT_TOKEN=<unique token>

# Agent's home channel
DISCORD_HOME_CHANNEL=<agent's channel ID>

# Only respond in these channels (whitelist)
DISCORD_ALLOWED_CHANNELS=<home channel>,<war-room>,<general>

# Free response in home channel (no @mention required)
DISCORD_FREE_RESPONSE_CHANNELS=<home channel>
```

## Debugging

```bash
# Check which gateways are running
systemctl --user list-units "hermes-gateway*" --no-pager

# Check slash command sync status
journalctl --user -u hermes-gateway-drey --no-pager | grep -i "sync\|slash\|command"

# Verify channel filtering env vars are loaded
grep DISCORD_ALLOWED_CHANNELS ~/.hermes/profiles/<agent>/.env

# Check logs for channel filtering
journalctl --user -u hermes-gateway-<agent> --no-pager | grep -i "allowed\|channel\|ignore"

# Diagnose duplicate slash commands (causes sync failure)
cd ~/.hermes/hermes-agent && source venv/bin/activate
python3 -c "
from hermes_cli.commands import COMMAND_REGISTRY, _is_gateway_available, _resolve_config_gates
config = _resolve_config_gates()
auto = {c.name.lower() for c in COMMAND_REGISTRY if _is_gateway_available(c, config)}
manual = {'new','reset','model','reasoning','personality','retry','undo','status','sethome','stop','compress','title','resume','usage','provider','help','insights','reload-mcp','voice','update','restart','approve','deny','thread','queue','background','btw'}
overlap = manual & auto
print(f'Auto-registered: {len(auto)}')
print(f'Manual: {len(manual)}')
print(f'Duplicates: {len(overlap)}')
if overlap:
    print(f'Remove these from _register_slash_commands(): {sorted(overlap)}')
"
```

## Known Issues (cont.)

### 4. Free Response Channels Not Working — Agents Still Require @mention
**Status:** Root cause found 04/18.

**Root Cause:** Each agent's `config.yaml` has `free_response_channels: ''` (empty string). The gateway code checks `self.config.extra.get("free_response_channels")` first — since empty string is NOT `None`, it never falls through to `os.getenv("DISCORD_FREE_RESPONSE_CHANNELS")` in the `.env`. The env var is silently ignored.

**Fix:** Set the correct channel ID directly in `config.yaml`:
```yaml
discord:
  free_response_channels: '<agent_dedicated_channel_id>'
```

**Verification:**
```bash
# Check if config.yaml is overriding env vars
grep "free_response_channels" ~/.hermes/profiles/<agent>/config.yaml
# If it shows `''` — that's the problem. Set it to the channel ID.
```

### 5. @everyone and Role Mentions Don't Trigger Agents
**Status:** Patched 04/18.

**Root Cause:** The gateway only checks `message.mentions` (direct user pings). Discord sends `@everyone` in `message.mention_everyone` and role pings in `message.role_mentions` — neither was checked.

**Fix:** Patch `gateway/platforms/discord.py` in three places:
1. **Bot filter** (~line 664): Allow `DISCORD_ALLOW_BOTS=mentions` to accept `@everyone`/role pings
2. **Multi-agent filter** (~line 686): Count `@everyone`/role as "self mentioned" so bots don't skip
3. **Require mention check** (~line 2971): Let `@everyone`/role satisfy the mention gate

**WARNING:** `hermes update` will overwrite these patches. Reapply after updating.

### 6. Gateway Shows Online But Doesn't Respond — Stuck Session
**Symptom:** Service status shows `active`, bot appears online in Discord, but doesn't respond to messages.

**Root Cause:** A previous API error (e.g., OpenRouter rate limit on free models) corrupted the session state. The gateway process is running but the session loop is stuck.

**Fix:** Hard kill and restart:
```bash
systemctl --user kill hermes-gateway-<agent>
sleep 1
systemctl --user reset-failed hermes-gateway-<agent>
systemctl --user start hermes-gateway-<agent>
```

## Lessons Learned (04/17-04/18)

- Restarting gateways is required after changing env vars — code changes alone won't fix runtime behavior
- If only ONE bot's slash commands appear in Discord, that bot has a stale cached sync; others failed to sync
- The 8000-byte limit is for the TOTAL command payload, not individual commands
- `--replace` flag on gateway start can conflict if old process is still running — auto-restart on failure handles this
- `config.yaml` values take priority over `.env` vars — always check config.yaml first when debugging missing env vars
- OpenRouter free models (z-ai/glm-4.5-air:free) are unreliable for agents — rate limits cause crashes that corrupt session state
- `hermes update` overwrites gateway patches (@everyone/role mention, DISCORD_REPLY_MENTION_ID) — reapply after updating
