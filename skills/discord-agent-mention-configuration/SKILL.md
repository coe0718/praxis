---
title: Discord Agent @Mention Configuration
name: discord-agent-mention-configuration
category: Multi-Agent Setup
description: Configure Discord agents to @mention Tuck appropriately based on context, not in every reply.
author: Tuck
created: 2026-04-16
tags: discord, agents, configuration, mentions, multi-agent
---

## Problem

Discord agents were @mentioning Tuck in every message, spamming the chat. This happened because:
1. Original SOUL.md rule required "@mention in every reply"
2. Discord platform interaction patterns weren't considered
3. Users wanted @mentions only for specific scenarios

## Solution

Update all Discord agent SOUL.md files with precise @mention rules:

### Key Changes

**Before (problematic):**
```markdown
## MANDATORY: @Mention Tuck In Every Reply

RULES:
1. When Tuck @mentions you, your response MUST include <@1493038094105055313> in the message. No exceptions.
2. When Tuck assigns you a task and you complete it, @mention Tuck in your completion message.
3. This applies to EVERY reply to Tuck, not just task completions. Short answers, long answers, everything.
```

**After (targeted):**
```markdown
## MANDATORY: @Mention Tuck When Appropriate

RULES:
1. When Tuck @mentions you, your response MUST include <@1493038094105055313> in the message. No exceptions.
2. When Tuck assigns you a task and you complete it, @mention Tuck in your completion message.
3. For regular conversation with Tuck (not @mentions, not task completions), do NOT include @mentions.
```

### Implementation Steps

1. **Read all agent SOUL.md files** to understand current configuration
2. **Update each agent's SOUL.md** with the corrected mention rules
3. **Test the behavior** after agents restart
4. **Monitor for proper @mention usage**

### Files to Update

- `~/.hermes/profiles/drey/SOUL.md`
- `~/.hermes/profiles/vex/SOUL.md`
- `~/.hermes/profiles/scout/SOUL.md`
- `~/.hermes/profiles/echo/SOUL.md`
- `~/.hermes/profiles/herald/SOUL.md`

## Best Practices

1. **Context-aware mentions**: Only @mention when necessary (replies to @mentions, task completions)
2. **Platform-specific rules**: Discord has different interaction patterns than other platforms
3. **User experience**: Avoid spam while maintaining necessary communication
4. **Clear documentation**: Explicit rules prevent confusion

## Troubleshooting

If agents still @mention incorrectly:
1. Check SOUL.md files for correct rules
2. Restart Discord agents to pick up new configuration
3. Verify Tuck's Discord bot ID is correct (1493038094105055313)
4. Test with different interaction scenarios

## Code-Level Issue (discovered 2026-04-17)

SOUL.md alone wasn't enough — there were **two layers** of @mention injection in the codebase, and the platform-level one was broken:

### Architecture: Two Mention Injection Points

1. **Gateway level** (`gateway/platforms/base.py`, lines ~1774-1779) — Correct.
   - Checks `event.source.user_id == _reply_mention_id` (Tuck's bot ID: `1493038094105055313`)
   - Only adds @Tuck when Tuck initiated the conversation
   - Agent responds to Agent → `event.source.user_id` is Agent, not Tuck → no mention ✓

2. **Platform level** (`gateway/platforms/discord.py`, `send()` and `edit_message()`) — **Redundant and buggy.**
   - Checked if `reply_to` message author == `_reply_mention_id`
   - Since `reply_to` is always set to the incoming message ID by the gateway, this fired every time an agent responded to a Tuck message — even when the gateway already handled it
   - Task completion check was dead code (wrapped in `if should_mention` which was only True when already replying to Tuck)

### Fix Applied

Removed all mention injection logic from `discord.py` `send()` and `edit_message()` methods. The gateway-level check in `base.py` is the single source of truth. After fixing, agents must restart:

```bash
for name in drey echo herald scout vex; do
  systemctl --user restart hermes-gateway-$name
done
```

### How to Verify

1. Tuck @mentions Agent A → Agent A should @mention Tuck back ✓
2. Agent A talks to Agent B in a thread → Agent B should NOT @mention Tuck
3. Agent completes a task → SOUL.md rules govern mention behavior (code won't auto-inject)

### Relevant Env Vars (set in each agent's `~/.hermes/profiles/<name>/.env`)

- `DISCORD_REPLY_MENTION_ID=1493038094105055313` — Tuck's bot user ID
- `DISCORD_ALLOW_BOTS=mentions` — Accept messages from other bots that @mention this agent
- `DISCORD_ALLOWED_USERS` — Must include all bot user IDs for multi-agent visibility

## Finding Bot User IDs & Sending Proper @Mentions

Discord mentions require the bot's **user ID**, not just the bot name. Text like `@Drey` won't ping — you must use `<@BOT_USER_ID>`.

### How to Get Bot User IDs

Discord bot tokens are `BASE64_ID.SECRET.HASH`. Decode the first segment:

```bash
for agent in drey vex scout echo herald; do
  token=$(grep DISCORD_BOT_TOKEN ~/.hermes/profiles/$agent/.env | cut -d= -f2)
  id_b64=$(echo $token | cut -d. -f1)
  echo "$agent: $(echo $id_b64 | base64 -d)"
done
```

### Current Bot IDs (as of 04/2026)

| Agent | Bot User ID |
|-------|-------------|
| Tuck | 1493038094105055313 |
| Drey | 1493042656484397116 |
| Vex | 1493043758487568544 |
| Scout | 1493043223105372160 |
| Echo | 1493044422596624451 |
| Herald | 1493044746774511676 |

### Proper Mention Format

```
<@1493042656484397116>   ← pings Drey
<@1493043758487568544>   ← pings Vex
```

Plain text like `@Drey` or `@Vex` does NOT work. Always use the `<@ID>` format.

### Prerequisites for Agents to Receive @Mentions from Tuck

Each agent's `.env` must have:
- `DISCORD_ALLOW_BOTS=mentions` — accept messages from other bots
- `DISCORD_ALLOWED_USERS` — must include Tuck's bot ID (`1493038094105055313`) and other agent bot IDs
- The channel must be in `DISCORD_ALLOWED_CHANNELS`

Without these, agents won't even see Tuck's messages in shared channels.

### Technical Bug: `message.mentions` Can Be Empty for Uncached Members

**Symptom:** Agent ignores messages with @mentions (e.g., `<@1493042656484397116>`) but sees the same message fine when the @mention is removed.

**Root Cause:** Discord.py's `message.mentions` list is populated from the *member cache*. If the bot hasn't cached the mentioned user yet, `message.mentions` returns empty even though `message.content` contains the `<@ID>` string. This happens when:
- The bot just started and hasn't seen the member in any guild yet
- The bot was restarted recently
- The member isn't in a shared visible channel

**Code Location:** `gateway/platforms/discord.py` line ~675:
```python
elif allow_bots == "mentions":
    if not self._client.user or self._client.user not in message.mentions:
        return  # FALSE POSITIVE — message.mentions is empty due to cache miss
```

**Why Removing @mention Works:** Without the @mention, the message bypasses the bot filter entirely and reaches the normal human-user allowlist check. Since Jeremy (human) is in `DISCORD_ALLOWED_USERS`, it processes normally. The bug only triggers when `DISCORD_ALLOW_BOTS=mentions` checks if "we were mentioned."

**Fix Options:**

1. **Parse raw content** (preferred): Check `message.content` for `<@{bot_id}>` pattern instead of trusting `message.mentions`:
   ```python
   # In the mentions filter:
   mentioned_in_content = self._client.user and f"<@{self._client.user.id}>" in message.content
   if not mentioned_in_content and self._client.user not in message.mentions:
       return
   ```

2. **Default to mentions only** (workaround): Since `mentions` is the default, change to `all` temporarily:
   ```bash
   DISCORD_ALLOW_BOTS=all  # Accepts ALL bot messages, not just @mentions
   ```
   **Security risk:** Other bots (including spam/rate-limit-annoyers) can now trigger this agent.

3. **Prime the cache** (manual): Have the agent fetch the member explicitly:
   ```python
   # In gateway startup, fetch self into cache
   await guild.fetch_member(self._client.user.id)
   ```

**Status:** No upstream fix yet — patch `discord.py` locally or use workaround.

## Gotcha: New Agents Always Need Tuck's Bot ID

When creating a new Discord agent (e.g., writer agents, aux agents), `DISCORD_ALLOWED_USERS` defaults to only the human users. **Tuck's bot ID (1493038094105055313) must be added manually** or Tuck cannot communicate with the agent at all — messages are silently dropped.

Symptoms: Tuck sends @mentions to the agent, agent is running, but never responds.

Fix in `~/.hermes/profiles/<agent>/.env`:
```
DISCORD_ALLOWED_USERS=<existing_user_ids>,1493038094105055313
```

Then restart:
```bash
systemctl --user restart hermes-gateway-<agent>
```

**This was missed for Kai (kai-voss) on 2026-04-21.** Don't forget for future agents.

## Gotcha: Thread File Routing

When an agent responds in a Discord thread, all output (text + file attachments) goes to the thread. If you want files to appear in the **main channel** (e.g., for easy mobile downloads), the agent must explicitly use `send_message` to post the file to the main channel separately.

### Pattern for File Output to Main Channel

Add to agent's SOUL.md:
```markdown
**After saving, send the file to the main channel:**
Use `send_message` with `target=discord:<main_channel_id>` and include the file path with MEDIA: tag:
```
Chapter X is done.
MEDIA:/absolute/path/to/file.md
```
Then tell the user in the thread: "Posted in the main channel."
```

This keeps threads clean for discussion and puts downloadable files where users expect them.

**Discovered 2026-04-21** with Kai — Malik preferred files in the main #kai-voss-writing channel for easier phone access.

## Related Skills

- [discord-agent-setup](discord-agent-setup): General Discord agent configuration
- [multi-agent-discord](multi-agent-discord): Multi-Agent Discord team setup
- [hermes-discord-agent-debug](hermes-discord-agent-debug): Debug Discord agent issues