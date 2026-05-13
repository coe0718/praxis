---
title: Multi-Agent Approval Checkpoint
name: multi-agent-approval-checkpoint
category: multi-agent
description: Prevent runaway multi-agent builds by requiring explicit approval before implementation.
author: Tuck
created: 2026-04-17
tags: multi-agent, coordination, workflow, approval
---
autoload: true

## Problem

When multiple agents are tasked with brainstorming or planning, they tend to skip the "wait for approval" step and jump straight to building. This happened during the Hermes Creative Hackathon (04/17/2026) — five agents were asked to pitch ideas, and instead of waiting for Jeremy to pick a winner, they started building immediately. The orchestrator (Tuck) made it worse by egging them on with "go go go."

## Root Cause

Agents have no built-in "stop and present" mode. They're wired to be action-oriented — given a task, they execute. The gap between "good idea" and "approved to build" doesn't exist in their default behavior.

## Solution: Approval Checkpoint in SOUL.md

Add this section to every agent's SOUL.md file:

```markdown
## APPROVAL CHECKPOINT — MANDATORY

Before writing ANY code, creating ANY files, or starting ANY implementation:

1. **Pitch your idea** — explain what you want to build and why
2. **Wait for approval** — do NOT proceed until Jeremy or Tuck explicitly says "go", "approved", "do it", or equivalent
3. **If unclear, ask** — "Should I start building?" is always a valid question
4. **Never build in a brainstorm** — discussion is not permission. Approval is permission.

Breaking this rule means you wasted time building something that might get scrapped. Respect Jeremy's time. Present the plan, get the green light, THEN build.
```

### Where to Add

Prepend this to the SOUL.md (top of file, before other rules):

```bash
CHECKPOINT='
## APPROVAL CHECKPOINT — MANDATORY
...
'

for agent in drey vex scout echo herald; do
  echo "$CHECKPOINT" | cat - ~/.hermes/profiles/$agent/SOUL.md > /tmp/soul_update
  mv /tmp/soul_update ~/.hermes/profiles/$agent/SOUL.md
done

# Restart to pick up changes
systemctl --user restart hermes-gateway-drey hermes-gateway-vex hermes-gateway-scout hermes-gateway-echo hermes-gateway-herald
```

## Orchestrator Rule — HARD LEARNED (04/17)

The orchestrator (Tuck) must also follow this pattern:

- Do NOT tell agents "go" or "build it" unless Jeremy has approved
- The orchestrator's job during brainstorming is to facilitate discussion and synthesize, not to drive execution
- After discussion, present a summary to Jeremy for approval before directing agents to build
- **Agents will @mention Tuck during discussion and ask for direction. Tuck must respond with "present your plan" or "waiting for Jeremy's approval" — NOT "go build."** This is the #1 failure mode. During the 04/17 hackathon test, Tuck kept responding to agent pings with "go" energy, which caused agents to skip approval entirely. The orchestrator must break the ping→push cycle.

## SOUL.md Changes Require Restart

Agents do NOT hot-reload SOUL.md. If agents are running when you update SOUL.md, they won't pick up the new rules until restarted. Always restart after making SOUL.md changes:

```bash
for agent in drey vex scout echo herald; do
  systemctl --user kill hermes-gateway-$agent  # kill, not stop
done
# Wait a moment, then start
systemctl --user start hermes-gateway-drey hermes-gateway-vex hermes-gateway-scout hermes-gateway-echo hermes-gateway-herald
```

## When to Use

- Any collaborative task where multiple agents contribute ideas
- Hackathon or project planning sessions
- Any time agents are working on something that might be scrapped or redirected

## When NOT to Use

- Simple, well-defined tasks with clear specifications (e.g., "fix this bug")
- When Jeremy explicitly says "just build it" upfront

## Testing Before Recording Demos

When the multi-agent workflow is being recorded for a demo (hackathon, showcase, etc.):

1. **Test in a secondary channel first** — Use #general or a test channel, NOT the primary #war-room. Keep the main channel clean.
2. **Run multiple tests** — First run will likely expose behavior issues (agents building without approval, orchestrator egging them on). Fix SOUL.md, restart, test again. Expect 3-4 runs before clean behavior.
3. **Verify mentions work** — Ensure `<@BOT_ID>` format pings actually reach agents. Plain `@Name` does NOT work in Discord.
4. **Orchestrator discipline** — Tuck must be a calm facilitator during demos, not a drill sergeant. No "go go go." Present, approve, then direct.
5. **Don't record until behavior is clean** — A demo of agents going rogue is worse than no demo. Get it right first, then record.

### Demo Recording Workflow

A compelling multi-agent demo structure:
1. Start on Telegram — Jeremy sends a casual message to Tuck
2. Tuck posts structured brief to Discord war room with proper `<@ID>` mentions
3. Agents reason, debate, push back (sped-up montage)
4. Agents present plan, wait for approval
5. Build montage — code written, assets generated
6. Final output plays
7. End card — "One message. Six agents. One finished product."

The killer moment is the Telegram → Discord handoff. One casual message triggers a full creative production pipeline.

## Killing Runaway Agents

**`systemctl --user stop` is NOT forceful enough.** During the 04/17 hackathon test, agents continued running after `stop` commands. Use `kill` instead:

```bash
# RECOMMENDED: Hard kill — actually stops them
for agent in drey vex scout echo herald; do
  systemctl --user kill hermes-gateway-$agent
done

# NOT RECOMMENDED: May leave agents running
systemctl --user stop hermes-gateway-drey hermes-gateway-vex ...
```

Always verify agents are dead:
```bash
systemctl --user list-units "hermes-gateway-*" --no-pager
# "failed" = stopped. "active" = still running.
```

## Shared Project Directory

When multiple agents work on the same project, they MUST write files to a shared directory. Otherwise code gets scattered across `~/.hermes/profiles/*/home/`.

```bash
mkdir -p /home/coemedia/projects/<project-name>
```

Include the full path in the task brief so all agents write to the same place.

## Discord @Mention Bot IDs

Plain `@Name` does NOT work in Discord. Must use `<@BOT_ID>`:

| Agent | Bot ID |
|-------|--------|
| Drey  | `<@1493042656484397116>` |
| Vex   | `<@1493043758487568544>` |
| Scout | `<@1493043223105372160>` |
| Echo  | `<@1493044422596624451>` |
| Herald| `<@1493044746774511676>` |
| Tuck  | `<@1493038094105055313>` |

## Related Skills

- [discord-agent-mention-configuration](discord-agent-mention-configuration): Agent mention rules and bot IDs
- [hermes-multi-agent-discord](hermes-multi-agent-discord): Multi-agent team setup
