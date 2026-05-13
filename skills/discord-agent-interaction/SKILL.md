---
title: Discord Agent Interaction
name: discord-agent-interaction
category: multi-agent
description: How to properly interact with, restart, and mention Discord agents
tags: [discord, agents, mentions, restart]
---
autoload: true

# Discord Agent Interaction Guide

## Mentioning Agents

Discord requires `<@USER_ID>` format for mentions, not `@name`.

**Bot IDs:**\n- Tuck: `1493038094105055313`\n- Drey: `1493042656484397116`\n- Scout: `1493043223105372160`\n- Echo: `1493044422596624451`\n- Vex: `1493043758487568544`\n- Herald: `1493044746774511676`\n- Kai-Voss: (not yet recorded)\n- Quill: `1500987036193001593` — created 05/04, resolve from token)\n\n**Human IDs:**\n- Jeremy: `669640444740763702`

**Example:** `<@1493042656484397116> here's your task` mentions Drey.

## Finding Bot IDs

Decode from Discord bot token. Token format: `BASE64_ID.SECRET.HASH`
```bash
echo $TOKEN | cut -d. -f1 | base64 -d
```

Or check each agent's `.env`:
```bash
grep DISCORD_BOT_TOKEN ~/.hermes/profiles/{agent}/.env | cut -d= -f2 | cut -d. -f1 | base64 -d
```

## Restarting Agents

Agent services are **user-level**, not system-level. Do NOT use `sudo`.

```bash
# Stop a single agent
systemctl --user stop hermes-gateway-{name}

# Start a single agent
systemctl --user start hermes-gateway-{name}

# Restart all agents (gateway agents with systemd services)\nfor agent in drey echo herald kai-voss quill scout vex; do\n  systemctl --user restart hermes-gateway-$agent\ndone
```

## Agent File Locations

- Config: `~/.hermes/profiles/{name}/.env`
- SOUL.md: `~/.hermes/profiles/{name}/SOUL.md`
- Agent code: `~/.hermes/profiles/{name}/home/`
- Skills: `~/.hermes/profiles/{name}/skills/`

## Known Behaviors

- Agents can be overeager — they may start building without explicit approval
- Vex tends to go rogue and write code while others are still planning
- Agents respond to Tuck's @mentions but need `DISCORD_ALLOW_BOTS=mentions` in their .env
- Loop risk: Drey+Echo can @mention-back each other in #general
- Agents only see messages in their `DISCORD_ALLOWED_CHANNELS`
- **⚠️ Profile agents resolve `~` to their own `home/` directory, not the system home.** When you tell an agent "put files in ~/projects/foo", they interpret it as `~/.hermes/profiles/{name}/home/projects/foo/` instead of `/home/coemedia/projects/foo/`. Always give absolute paths (`/home/coemedia/projects/...`) and **verify files landed where expected** — the profile `home/` directory is the most common place to find "missing" files. If files are in the wrong place, move them to the real project root and tell the agent the correct absolute path for subsequent files.
- **@everyone bug (PATCHED 04/24):** Agents don't respond to `@everyone`. Root cause: Discord API sets `message.mention_everyone=True` for `@everyone` but does NOT populate `message.mentions` with bot IDs. The gateway's mention check (`self._client.user not in message.mentions`) returns False for @everyone. **Fix:** Three patches in `~/.hermes/hermes-agent/gateway/platforms/discord.py` that check `getattr(message, "mention_everyone", False)` and treat it as a self-mention. See "@everyone Patch Details" section below.
- **⚠️ Patch fragility:** The discord.py patches get WIPED whenever the gateway file is overwritten (restart, package update, `hermes update`). File was last overwritten 04/23 at 16:21. After any gateway update, verify patches are present: `grep -c mention_everyone gateway/platforms/discord.py` should return 4+. If 0, re-apply.

## Anti-Loop Debate Rules (added 04/20)

Agents that can @mention each other (council agents, or execution agents when explicitly asked to collaborate) need hard stopping points to prevent infinite loops.

**Rules added to all 8 agent SOUL.md files (Drey, Scout, Vex, Echo, Herald, Sable, Locke, Maren):**

1. **Turn budget: 3 exchanges max per topic.** After 3rd response, summarize position and stop.
2. **Don't repeat yourself.** If argument didn't land, don't restate it. Summarize both sides.
3. **Escalate on deadlock.** No consensus after 3 turns → report disagreement to Jeremy with both positions in 2-3 sentences each.
4. **One topic at a time.** Don't branch into side debates.

**For execution agents (Drey, Scout, Vex, Echo, Herald):**
- Default: don't respond to other agents unless Jeremy/Tuck explicitly asks for collaboration
- When collaboration is triggered: debate rules apply

**For council agents (Sable, Locke, Maren):**
- They're designed to debate — mentions are mandatory for the council to function
- Debate limits added to keep debates finite

**04/20 lesson:** SOUL-level rules alone are NOT sufficient to prevent loops. Council agents continued looping despite turn budget rules because: (1) agents don't track their own turn count across messages, (2) each message is processed independently, (3) SOUL guidelines can be overridden by the agent's core role (debate protocol takes priority). The next step is code-level enforcement: gateway tracks turn counts per conversation thread and injects a "DEBATE ENDED" system message after N exchanges.

## Killing Runaway Agents

**Use `stop`, not `kill`.** `systemctl --user kill` sends SIGTERM but systemd's auto-restart brings them right back. `stop` actually disables the service.

```bash
# Stop all agents (this actually works)
systemctl --user stop hermes-gateway-{drey,scout,vex,echo,herald}

# Verify they're dead
systemctl --user list-units "hermes-gateway-*" | grep -E "active|failed"
```

If `stop` isn't enough, force-kill the Python processes directly:
```bash
ps aux | grep hermes-gateway | grep -v grep | awk '{print $2}' | xargs kill -9
```

**⚠️ `--replace` signal propagation hazard:** When starting a gateway with `--replace`, it reads the PID from `gateway_state.json` and sends SIGTERM. If the old process is already dead, the same PID may have been recycled by a newly-started process, killing THAT instead. This means:
- Starting Vex's gateway with `--replace` can kill Tuck's main gateway if PID reuse coincidentally matches
- Starting a gateway in a background shell process with `--replace` can kill the shell wrapper instead of the intended target
- Multiple rapid restarts increase the risk because PID numbers are more likely to overlap

**Safer manual restart pattern:**
```bash
# 1. Kill old gateway manually by PID
kill $(ps aux | grep "python.*profile <name> gateway" | grep -v grep | awk '{print $2}') 2>/dev/null
sleep 2

# 2. Verify dead
ps aux | grep "python.*profile <name>" | grep -v grep || echo "dead"

# 3. Start fresh WITHOUT --replace
cd ~/.hermes/hermes-agent && source venv/bin/activate && \
  python -m hermes_cli.main --profile <name> gateway run
```

**⚠️ Slash command rate limiting:** Every gateway restart triggers a `PUT /applications/{id}/commands` call to Discord. This endpoint is rate-limited to ~200 requests/day per bot application. After too many restarts in a 24-hour window, ALL slash commands disappear from Discord with the log message `Slash command sync timed out — Discord rate-limit bucket may be saturated; will retry on next reconnect`. The rate limit is per-bot-application, not per-guild — one bot's rate limit exhausts all commands for that bot. **Fix: don't restart the same gateway more than 3-4 times in 24 hours.** If commands are missing, wait 30-60 minutes for the bucket to recover. The gateway retries on reconnect automatically.

**04/17 lesson:** `kill` worked when agents were mid-execution (04/17 hackathon).
**04/18 lesson:** `kill` failed because systemd auto-restart brought them back instantly. `stop` was the fix.

### Council Agents (sable, locke, maren) — NOT systemd

Council agents run as **direct processes** with `--profile`, NOT as systemd services. You cannot stop them with `systemctl --user stop`.

```bash
# Find council agent processes
ps aux | grep -E "sable|locke|maren" | grep "gateway run"

# Kill by PID directly
kill <PID>

# Example from 04/18:
# coemedia 3589638 ... python -m hermes_cli.main --profile sable gateway run --replace
# coemedia 3589699 ... python -m hermes_cli.main --profile locke gateway run --replace
# coemedia 3589751 ... python -m hermes_cli.main --profile maren gateway run --replace
```

**Why they're different:** Council was started manually (not via systemd units), so systemctl doesn't manage them. They have no auto-restart — `kill` actually works for these.

## Free Response Channels

To let agents respond without @mentions, add the channel ID to `DISCORD_FREE_RESPONSE_CHANNELS` in their `.env` file. Don't try config.yaml — dotenv doesn't reload it reliably.

```bash
# Example: allow council channel free response
echo 'DISCORD_FREE_RESPONSE_CHANNELS=1483967788342050848,CHANNEL_ID_HERE' >> ~/.hermes/profiles/{agent}/.env
```

⚠️ **Rate limiting risk:** Multiple agents in the same free-response channel can trigger Discord rate limits quickly. Test with 1-2 agents first.

## Agent Not Responding to Tuck (learned 04/21)

If an agent doesn't respond to Tuck's @mentions, check `DISCORD_ALLOWED_USERS` in their `.env`. Tuck's bot ID must be listed:

```bash
# Check current allowed users
grep DISCORD_ALLOWED_USERS ~/.hermes/profiles/{agent}/.env

# Add Tuck's bot ID (1493038094105055313)
# Edit .env and add/append to DISCORD_ALLOWED_USERS
```

**Symptom:** Agent is running, responds to human users, but ignores Tuck's messages silently.
**Cause:** Tuck's bot ID not in `DISCORD_ALLOWED_USERS` — agent filters out bot-to-bot messages by default.
**Fix:** Add `1493038094105055313` to the agent's `DISCORD_ALLOWED_USERS` env var, then restart.

## ⚠️ Output Length Limits: Discord's 2000-Character Cap (learned 05/01)

Discord has a **hard 2000-character limit** per message. Agents that generate large outputs (full website code, long analysis, multi-file reviews) get **silently truncated** — the user sees `⚠️ Response truncated due to output length limit` instead of the actual content.

**This is the #1 most frustrating behavior for users.** It looks broken, not helpful.

### How It Happens

Agent is asked to "rebuild the whole website" → agent generates complete HTML/CSS as a single Discord message → Discord cuts it off at 2000 chars → user sees a wall of partial code followed by the truncation warning → user has to ask again, often multiple times.

### The Fix: Agents Must Self-Limit

Add these rules to every agent SOUL.md, especially **Drey** (most prone to verbosity):

```markdown
## Discord Output Limits (CRITICAL)

1. Discord messages are capped at **2000 characters**. If your response exceeds this, you MUST handle it — do NOT let it get truncated.
2. For large output (code, designs, full files): **write to a file** on the filesystem, then provide the path and a brief summary. Never paste raw HTML/CSS/markdown code directly into Discord.
3. For long explanations: **trim to essentials**. If it genuinely needs more space, split into multiple short messages (max 3 sequential messages).
4. If you generate a website or app output: save it as an HTML file, tell the user where it is, and optionally send the file as a Discord attachment using `MEDIA:/path/to/file` in your response.
```

### Pattern for Large Output Delivery

```markdown
# Bad (gets truncated):
[3000 chars of HTML...]
⚠️ Response truncated due to output length limit

# Good (works every time):
The redesign is done. Key changes:
- New hero section with bolder messaging
- Simplified feature grid (6→4 boxes)
- Purple accent throughout

The full site is at /home/coemedia/projects/archiview-redesign/index.html
MEDIA:/home/coemedia/projects/archiview-redesign/index.html
```

### Agent SOUL.md Update Quick Reference

Check if an agent's SOUL.md already has output limit rules:
```bash
grep -c "2000\|Discord.*limit\|truncat" ~/.hermes/profiles/{agent}/SOUL.md
# Returns 0 = needs updating
```

**Known offenders (as of 05/01):**
- Drey — most frequent, asked to build/design things, generates large output
- Vex — second most frequent, verbose in code reviews
- Scout — occasional, long research reports

### Enforcing at the Code Level (Future)

For a permanent fix, the gateway could:
1. Automatically detect responses >1900 chars and save them as Discord file attachments instead of text messages
2. Inject a "platform_limit" system message that agents see before generating
3. Store long responses to a temp file and auto-attach

No code-level enforcement exists yet (05/01). SOUL.md rules are the current defense.

## File Delivery: Thread vs Main Channel (learned 04/21)

When an agent responds in a Discord thread, ALL content (text + file attachments) goes to the thread. If you want files delivered to the main channel while keeping conversation in the thread:

**Approach:** Have the agent use `send_message` with `target=discord:CHANNEL_ID` and include `MEDIA:/path/to/file` in the message body. The gateway strips MEDIA: tags and sends the file as a Discord attachment.

Example in agent SOUL.md:
```
After saving a file, send it to the main channel:
Use send_message with target=discord:1495933260608442560 and include:
MEDIA:/full/path/to/file.md
Then tell the user in the thread: "Posted in the main channel."
```

**Why:** This keeps threads clean for discussion and puts downloadable files where users can easily access them (especially on mobile).

## Setting Up a New Agent Profile (learned 05/04)

Complete workflow for standing up a brand new Hermes agent from scratch (e.g., Quill).

### Step 1: Create the Profile

```bash
hermes profile create quill
```
This creates `~/.hermes/profiles/quill/` with default SOUL.md, skills symlinks, and directories.

### Step 2: Write the SOUL.md

Professional tone, clear role definition, core principles, documentation types, and team context. Reference related agents (Tuck delegates, Drey codes, Vex reviews).

### Step 3: Write the Config (config.yaml)

Model on an existing agent (Scout's is well-structured). Key settings:
- `model.default` — the free model for this agent (e.g., `nvidia/nemotron-3-super-120b-a12b:free`)
- `model.provider` — `openrouter` if using OpenRouter
- `discord.require_mention: true` — agents only respond to @mentions
- `discord.allowed_channels` — the agent's channel ID
- `discord.free_response_channels` — same channel ID for auto-response
- `discord.reactions: false` — less noise for new agents
- `memory.provider: mnemosyne` — matches the fleet standard
- `terminal.cwd: .` — avoid the "messages" directory bug (Scout trap)

**Common pitfall:** The initial `hermes profile create` generates an empty config.yaml. The agent won't crash without it — it uses defaults — but Discord auth won't work until you explicitly set the config keys above.

### Step 4: Set Up Environment (.env)

The `.env` file is where Discord access control lives. This is the most critical and most commonly wrong file.

**Required vars for every new agent:**

```bash
# Discord bot token (from Developer Portal)
DISCORD_BOT_TOKEN=MTUwMDk4NzAzNjE5MzAwMTU5Mw.xxx

# Who can talk to the agent — Jeremy's user ID + all bot IDs
DISCORD_ALLOWED_USERS=669640444740763702,1493042656484397116,1493043223105372160,1493044422596624451,1493038094105055313,1493043758487568544,1493044746774511676

# Channel the agent lives in
DISCORD_ALLOWED_CHANNELS=1500988241476849814
DISCORD_FREE_RESPONSE_CHANNELS=1500988241476849814

# Let Tuck (and other bots) mention the agent
DISCORD_ALLOW_BOTS=mentions
```

**⚠️ Critical: Keys go in `.env`, not `auth.json`.** The `auth.json` credential pool is regenerated by the gateway at startup. Manually-inserted credentials in `auth.json` get wiped when the gateway restarts. Put ALL API keys (OpenRouter, Discord token, etc.) in `.env` — the gateway reads those at startup and populates the credential pool automatically.

**⚠️ Critical:** Leave `DISCORD_ALLOWED_USERS` out and the agent will log `Unauthorized user: 669640444740763702 (coe0718)` and ignore every message — including from the person who set it up. This is the #1 new-agent fails-to-respond bug.

**All known Discord IDs (as of 05/04):**
- Jeremy (human): `669640444740763702`
- Tuck: `1493038094105055313`
- Drey: `1493042656484397116`
- Scout: `1493043223105372160`
- Echo: `1493044422596624451`
- Vex: `1493043758487568544`
- Herald: `1493044746774511676`

### Step 5: Set Up Credentials (auth.json)

The auth.json stores API keys in credential pools. Copy the pattern from an existing agent (e.g., Drey's):

```json
{
  "version": 1,
  "active_provider": "openrouter",
  "credential_pool": {
    "openrouter": [{
      "id": "quill_or_1",
      "label": "OPENROUTER_API_KEY",
      "auth_type": "api_key",
      "priority": 0,
      "source": "env:OPENROUTER_API_KEY",
      "access_token": "sk-or-...",
      "last_status": "ok"
    }],
    "discord": [{
      "id": "quill_discord_1",
      "label": "DISCORD_BOT_TOKEN",
      "auth_type": "bot_token",
      "priority": 0,
      "source": "env:DISCORD_BOT_TOKEN",
      "access_token": "MTUwMDk4NzAzNjE5Mw..."
    }]
  },
  "providers": {"openrouter": {}}
}
```

Without both OpenRouter and Discord credentials in the pool, the gateway will crash immediately after the "Gateway Starting" banner with exit status 1. Both are required.

### Step 6: Create the Systemd Service

```bash
cat > ~/.config/systemd/user/hermes-gateway-quill.service << 'EOF'
[Unit]
Description=Hermes Agent Gateway - Quill (Documentation Specialist)
After=network.target
StartLimitIntervalSec=600
StartLimitBurst=5

[Service]
Type=simple
ExecStart=/home/coemedia/.hermes/hermes-agent/venv/bin/python -m hermes_cli.main --profile quill gateway run --replace
WorkingDirectory=/home/coemedia/.hermes/hermes-agent
Environment="HERMES_HOME=/home/coemedia/.hermes/profiles/quill"
Restart=on-failure
RestartSec=30
RestartForceExitStatus=75
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=60
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=default.target
EOF
```

Then enable and start:
```bash
systemctl --user daemon-reload
systemctl --user enable hermes-gateway-quill.service
systemctl --user start hermes-gateway-quill.service
```

### Step 7: Verify

```bash
# Check if running
systemctl --user status hermes-gateway-quill.service

# Check for unauthorized user errors
journalctl --user -u hermes-gateway-quill.service --since "1 minute ago"

# Test with a chat query
hermes --profile quill chat -q "Hello, are you operational?"
```

### Step 8: Create the Discord Channel

Jeremy creates the `#quill` channel in the Discord server and invites the bot via the OAuth URL from the Developer Portal (OAuth2 → URL Generator → bot scope + Send Messages/Read History permissions). The invite is a one-time action — after that the bot stays in the server.

### Profile Directory Layout (Quill Example)

```
~/.hermes/profiles/quill/
├── config.yaml          # Model, Discord channels, tools, memory, terminal
├── SOUL.md              # Personality, role, principles, team context
├── .env                 # Discord token + allowed users/channels/bots
├── auth.json            # Credential pools (OpenRouter, Discord)
├── skills/              # Symlinks to global skills
├── sessions/            # Chat transcripts
├── logs/                # Gateway logs
├── home/                # Agent's working directory
├── mnemosyne/           # Memory store
└── ...
```

### Common Pitfalls When Setting Up a New Agent

| Symptom | Cause | Fix |
|---------|-------|-----|
| Agent logs "Unauthorized user" | `DISCORD_ALLOWED_USERS` missing from `.env` | Add Jeremy's user ID |
| Gateway exits with status=1 immediately | Missing Discord token or OpenRouter key in auth.json | Populate credential pools |
| Agent starts but doesn't respond | OAuth invite not completed | Visit OAuth URL from Developer Portal |
| Agent ignores Tuck's messages | `DISCORD_ALLOW_BOTS=mentions` not set | Add to `.env` |
| "Provider resolver returned empty API key" | OpenRouter key not in auth.json credential pool | Add OpenRouter credential |
| Agent responds but generates trash | Wrong model, no doc-writing skill loaded | Set `model.default` correctly + preload skill in config |

## Sending Messages to Agent Channels (learned 04/24, updated 05/04)

When Tuck needs to send a task to another agent's Discord channel:

1. **Always call `send_message list` first** to get the correct target format (e.g., `discord:#scout`, `discord:#drey`). Don't guess channel IDs.
2. **Always use `<@BOT_ID>` mentions (e.g., `<@1493042656484397116>`) when the message needs the agent's attention.** This is not optional — agents do NOT see messages in their own channel without a proper mention. The `<@BOT_ID>` format works: Jeremy confirmed the agent sees it. Plain `@name` mentions (e.g., `@Drey`) do NOT work — they render but don't trigger the gateway's mention detection.
3. **Some agents don't respond to Tuck at all** regardless of channel or mention. If an agent ignores the message, it's likely `DISCORD_ALLOWED_USERS` doesn't include Tuck's bot ID (see above), or the agent's config simply doesn't allow bot-to-bot triggering. In that case, tell Jeremy to send the task directly, or do the work yourself.

**Correct approach:**
```
send_message target=discord:#drey message="<@1493042656484397116> Jeremy needs X done now"
```

**Wrong approaches:**
- `send_message target=discord:1483967982752235620` — sends to Tuck's own home channel, not the target agent
- Sending without `<@BOT_ID>` mention — agent will NOT see it. Plain `@Drey` or `@name` mentions are not detected by the gateway. Only `<@USER_ID>` format works.
- Assuming agent will see it just because it's "their" channel — they need the ping regardless

## @everyone Patch Details (applied 04/24, re-applied after wipes)

**File:** `~/.hermes/hermes-agent/gateway/platforms/discord.py`

**Three patches needed:**

1. **Multi-agent filter (~line 692):** The block is guarded by `message.mentions` — @everyone makes this empty, so the whole filter is skipped. Fix: also guard on `_everyone_ping = getattr(message, "mention_everyone", False)`. When True, set `_self_mentioned = True` and bypass the early-return checks.

2. **require_mention check (~line 3050):** `self._client.user not in message.mentions` is False for @everyone. Fix: add `_everyone_ping` check before this condition — if @everyone was used, skip the mention requirement.

3. **Bot allow_bots=mentions (~line 674):** Same issue — `self._client.user not in message.mentions` rejects @everyone. Fix: add `_everyone_ping` check.

**Quick verification after any gateway update:**
```bash
grep -c mention_everyone ~/.hermes/hermes-agent/gateway/platforms/discord.py
# Should return 4+. If 0, patches were wiped.
```

**Restart all agents after patching:**
```bash
for agent in drey echo herald kai-voss scout vex; do
  systemctl --user restart hermes-gateway-$agent
done
```

**Long-term fix:** These patches should be upstreamed to Hermes or moved to a post-install hook so they survive updates.

## Model Distribution Across Providers (added 04/20)

Council agents (and any agents that debate/collaborate) should be on **different free providers** to avoid shared rate limit buckets. Three agents all on the same free model = instant rate limit.

**Current council model assignments:**
- Sable: `nvidia/nemotron-3-super-120b-a12b:free` (NVIDIA bucket)
- Locke: `openai/gpt-oss-120b:free` (OpenAI bucket)
- Maren: `minimax/minimax-m2.5:free` (MiniMax bucket)

Config: `~/.hermes/profiles/{agent}/config.yaml` — change `model.default` key. Each provider has its own rate limit bucket (~40 RPM for NIM). Spreading agents = 3x available RPM.
