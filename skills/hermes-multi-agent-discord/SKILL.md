---
name: hermes-multi-agent-discord
description: Set up a multi-agent Hermes team on a single Discord server with profile-based orchestration. Each agent gets its own profile, bot token, channel, and personality.
version: 1.1
---

# Multi-Agent Discord Setup with Hermes Profiles

Run multiple Hermes agents as separate Discord bots in one server. Tuck orchestrates, specialist agents execute. Each agent is a fully independent Hermes profile.

## Architecture

- 1 Discord server with dedicated channels per agent + shared #war-room
- Each agent = 1 Hermes profile = 1 Discord bot token = 1 systemd service
- Tuck (orchestrator) delegates to specialists via @mention in #war-room
- Agents only respond in their channel and #war-room

## Channel Layout

| Channel | Who | Purpose |
|---------|-----|---------|
| #tuck | You + Tuck | Main interface, give high-level intent |
| #war-room | You + Tuck + all agents | Tuck delegates here via @mention |
| #drey | You + Drey | Direct access to coding agent |
| #scout | You + Scout | Direct access to research agent |
| #vex | You + Vex | Direct access to code review agent |
| #echo | You + Echo | Direct access to social media agent |
| #herald | You + Herald | Direct access to news/monitoring agent |
| #kai-voss-writing | You + Kai-Voss | Direct access to fiction writer agent |
| #quill | You + Quill | Direct access to technical documentation agent |
| #council | You + Sable/Locke/Maren | Deliberation council (debate/decision agents) |

## Current Agent Lineup

| Agent | Role | Model | Provider |
|-------|------|-------|----------|
| Tuck | Orchestrator / Main Assistant | deepseek-v4-flash (Api.select.ax) | Custom: select |
| Drey | Coding Specialist | minimax-m2.5 (free NIM) | NVIDIA NIM |
| Scout | Research & Reconnaissance | minimaxai/minimax-m2.5:free | OpenRouter |
| Vex | Code Review & Quality | deepseek-v4-flash | Custom: select |
| Echo | Social Media & Communications | nvidia/nemotron-3-super-120b-a12b:free | OpenRouter |
| Herald | News & Monitoring | nvidia/nemotron-3-super-120b-a12b:free | OpenRouter |
| Kai-Voss | Writer (Fiction) | kimi-k2.6 | Custom: select |
| Quill | Technical Documentation | nvidia/nemotron-3-super-120b-a12b:free | OpenRouter |
| Sable | Agitator (Council) | openai/gpt-oss-120b:free | OpenRouter |
| Locke | Skeptic (Council) | nvidia/nemotron-3-super-120b-a12b:free | OpenRouter |
| Maren | Arbiter (Council) | minimax/minimax-m2.5:free | OpenRouter |
| Hart | Moderator (Council) | PENDING | — |

## Pitfalls

- Bot tokens are sensitive — dont paste in chat. Regenerate after setup.
- echo conflicts with /usr/bin/echo — no wrapper alias possible. Use hermes -p echo.
- Each profile is fully independent — skills, memory, SOUL.md dont share unless explicitly copied.
- When editing config.yaml programmatically, regex replacements can leave artifacts from old sections — always verify the output. The safest approach for model changes is to copy the entire default config.yaml fresh and only modify the model block, rather than trying to regex-replace sections.
- hermes profile create --clone copies config.yaml, .env, and SOUL.md from the active profile — may need cleanup of credentials/models that shouldnt be shared. Remove Telegram config from Discord-only profiles.
- **Privileged Intents**: All bots need Presence Intent, Server Members Intent, and Message Content Intent enabled in Discord Developer Portal — Bot tab — Privileged Gateway Intents. Without these, gateways crash immediately with PrivilegedIntentsRequired.
- **Systemd restart throttle**: If gateways crash and restart too fast, systemd hits start-limit-hit. Run systemctl --user reset-failed before retrying, and stagger restarts with sleep 2 between each.
- **Connection timeouts**: Some gateways may timeout on first Discord connection (30s default). They usually succeed on systemd auto-restart. Check systemctl --user is-active for final status, not just first-attempt logs.
- hermes profile list may show stopped even when systemd services are running. Check systemctl --user is-active hermes-gateway-NAME for real status.
- When copying config.yaml for OpenRouter profiles, remove the custom_providers block entirely — its not needed and regex cleanup is error-prone.
- **personality override in config.yaml**: Cloned profiles inherit personality: kawaii (or whatever the default has) which overrides SOUL.md identity. Remove the personality: line from all agent profiles config.yaml or agents will ignore their SOUL.md and use the default personality instead.
- **Clear sessions after identity changes**: If an agent had wrong identity (e.g. Im Tuck), the old session context persists even after fixing SOUL.md. Delete sessions/sessions.json and state.db in the profile directory, then restart the gateway.
- **DISCORD_FREE_RESPONSE_CHANNELS vs DISCORD_ALLOWED_CHANNELS**: FREE_RESPONSE = agent responds to every message automatically. ALLOWED = agent can see the channel but only responds when @mentioned. War-room and general channels should be in ALLOWED only, not FREE_RESPONSE. Dedicated agent channels go in FREE_RESPONSE.
- **auto_thread: false recommended** for multi-agent setups. auto_thread=true causes agents to auto-create threads on every @mention, leading to fragmented conversations. Set auto_thread: false in all agent config.yaml files.
- **DISCORD_REPLY_MENTION_ID env var**: Set in each agents .env to Tucks bot ID (1493038094105055313). Auto-injects TUCK_ID into outgoing Discord responses when the last sender in that channel was Tuck. This is gateway-level enforcement.
- **Memory persistence**: All agent SOUL.md files should include a Memory Persistence section instructing agents to save task outcomes and preferences using the memory tool after each session. Without this, every session starts blank and agents cant recall past work.
- **Vex needs step-by-step instructions** for large tasks. If given a single open-ended instruction, it burns through context before writing results. Break tasks into numbered steps and tell it to save partial results.
- **Discord response path**: Discord uses streaming responses via GatewayStreamConsumer which calls adapter.send_message and adapter.edit_message directly. It does NOT go through the base adapters _process_message_background. To intercept/modify outgoing Discord messages, patch gateway/platforms/discord.py send_message method.
- **hermes update risk**: Both the discord.py patches will be overwritten by hermes update. Re-apply after updating, or consider upstreaming the patches.
- **auto_thread: true in config.yaml** makes bots auto-create Discord threads for every @mention. Inherited by all cloned profiles. Set to false if you dont want thread-based conversations. Change in config.yaml then restart gateways.
- **@everyone and role mentions dont trigger bots by default**: The Discord gateway only checks message.mentions (user mentions). Role pings go to message.role_mentions and @everyone goes to message.mention_everyone — both ignored by the stock gateway. Patch gateway/platforms/discord.py in three places: (1) bot filter DISCORD_ALLOW_BOTS=mentions check (~line 664), (2) multi-agent filter (~line 686), (3) require_mention check (~line 2971). All three need to also check message.mention_everyone and bool(message.role_mentions).
- **Adding channels later**: To add a new channel (e.g. #general) to all agents, append the channel ID to DISCORD_ALLOWED_CHANNELS in each profiles .env, then restart all gateways.
- **Bot-to-bot @mention communication IS possible** with two config changes:
  1. DISCORD_ALLOW_BOTS=mentions in each profiles .env — tells the gateway to accept messages from other bots when @mentioned (default is none which blocks all bot messages)
  2. All bot user IDs in DISCORD_ALLOWED_USERS — the _is_allowed_user check runs BEFORE the bot filter, so even with DISCORD_ALLOW_BOTS=mentions, bot messages get silently dropped unless their Discord user IDs are in the allowed list. Add all bot IDs plus the humans ID.
- **SOUL.md instructions must be forceful**: Soft instructions like you MUST do X get ignored by agents. Use stronger framing: MANDATORY, non-negotiable system rule, This is not optional, If you skip this, that is a failure. Label sections with MANDATORY in the heading.
- **Large tasks lose context**: Agents on GLM-5.1 burn through context fast on large codebases. When assigning a review or multi-step task via Discord, give numbered step-by-step instructions AND explicitly say save the file even if you only get partway through. Without incremental saves, the agent loses all work when context fills up.
- **Cross-agent task reporting**: To have agents report back to Tuck when assigned tasks, add a MANDATORY section to each agents SOUL.md requiring them to include Tucks bot ID mention in every reply when Tuck @mentions them.
- Session isolation: Each agent session is independent. When an agent completes work in session A and you message it later, its in session B with no memory. This is why SOUL.md instructions (loaded every session) are critical.
- **Agent memory persistence**: By default, non-Tuck agents dont save anything to memory across sessions. Add a Memory Persistence section to each agents SOUL.md instructing them to save task outcomes and learned preferences using the memory tool.
- **Don't delegate large tasks to Discord agents**: GLM-5.1 burns through context fast on complex multi-file tasks. For large tasks: do it yourself (Tuck via delegate_task with terminal/file toolsets), or break the task into small pieces (one product/crate at a time) when delegating to Discord agents.

## Adding a New Agent

See `references/adding-new-agent.md` for the complete step-by-step workflow covering profile creation, SOUL.md, config.yaml, auth setup, Discord channel creation, systemd service, bot invite, and testing. This reference was generated from the Quill addition (2026-05-04).
- `free_response_channels: ''` in config.yaml OVERRIDES .env: When you clone a profile, config.yaml gets `free_response_channels: ''`. The gateway code checks `self.config.extra.get("free_response_channels")` FIRST — if not None, the env var DISCORD_FREE_RESPONSE_CHANNELS is NEVER consulted. Empty string is not None, so config always wins. Fix: set the channel ID in config.yaml (`free_response_channels: 'CHANNEL_ID'`) for each agent, OR remove the key entirely so it falls through to the env var. Symptoms: agents dont respond freely in their dedicated channels even though .env has correct DISCORD_FREE_RESPONSE_CHANNELS. **CRITICAL**: `free_response_channels` must be at the TOP LEVEL of config.yaml, NOT nested under `agent:`. The code reads `self.config.extra.get()` which only sees top-level keys. Nested keys under `agent:` are silently ignored. Fixed 2026-04-18.
- **OPENROUTER_API_KEY not copied on clone**: `hermes profile create --clone` copies .env but the OPENROUTER_API_KEY line may end up commented out or missing. Always verify `grep "OPENROUTER_API_KEY=sk-" PROFILE/.env` after cloning. If missing, copy from a working profile's .env. The gateway will crash on first API call with "No LLM provider configured" if this is missing.
- **Council agents need SOUL.md cross-mention instructions**: Even with DISCORD_ALLOW_BOTS=mentions, council agents won't see each other's messages unless they @mention each other. Add a MANDATORY section to each council agent's SOUL.md with the exact bot IDs:
  ```
  When you post in #council, you MUST @mention the other council members:
  - When responding to Locke, include <@LOCKE_BOT_ID>
  - When responding to Sable, include <@SABLE_BOT_ID>
  - When responding to Maren, include <@MAREN_BOT_ID>
  ```
  Without this, agents post responses that the others never see, breaking the debate flow.

## Deliberation Council (Pure-Reasoning Agents)

A pattern for debate/decision agents with NO tools — just distinct thinking styles defined in SOUL.md.

Three agents: **Locke** (Skeptic), **Sable** (Agitator), **Maren** (Arbiter).

| Agent | Role | Thinking Style |
|-------|------|----------------|
| Locke | Skeptic | Finds flaws, challenges assumptions, stress-tests ideas |
| Sable | Agitator | Pushes boundaries, proposes ambitious alternatives |
| Maren | Arbiter | Weighs tradeoffs, synthesizes debate, makes final call |

### Key Config Differences from Execution Agents

- **toolsets: []** in config.yaml — no tools, pure reasoning
- **tool_use_enforcement: disabled** in config.yaml
- **agent.max_turns: 15** — short turns, debate is punchy
- SOUL.md: Define debate protocol, communication style, council workflow
- All council bot IDs + human + Tuck in DISCORD_ALLOWED_USERS
- **DISCORD_ALLOW_BOTS=mentions** so they respond to each other
- Shared #council channel in DISCORD_ALLOWED_CHANNELS (not free_response)
- Dedicated channels in DISCORD_FREE_RESPONSE_CHANNELS for 1:1 access
- Council channel requires @mention (deliberate invocation)

### Workflow

1. Human posts a question/problem in #council, @mentioning all three
2. Locke and Sable debate (bots respond to each other via DISCORD_ALLOW_BOTS=mentions)
3. Maren synthesizes and makes the final call
4. Maren output is the brief for execution agents (Drey, Vex, Scout)

### SOUL.md Pattern for Council Agents

Council agents need explicit debate protocol:
- When to speak vs. when to wait
- How to respond to other agents (reference by name, not just reply)
- Sign messages with [Locke], [Sable], [Maren]
- Keep responses under 300 words — debate is punchy, not essays
- Respect Maren final decisions even when you disagree
- No tool usage — pure reasoning only
