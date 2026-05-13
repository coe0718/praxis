---
name: discord-council-debate-fix
description: Fix looping, repetitive, or degenerating multi-agent council debates on Discord. Applies structured debate patterns, history injection, moderator roles, and response format enforcement.
version: 0.1.0
created: 2025-04-25
---
autoload: true

# Discord Council Debate Fix

## Problem
Council agents (Sable, Locke, Maren) loop, repeat arguments, or produce low-quality discussion because:
1. **No shared state** — Relay-style comms mean each agent only sees the previous message, not the conversation
2. **No consensus marking** — No mechanism to declare topics "settled"
3. **No moderation** — Nobody summarizes and redirects
4. **Free models** — Weaker models repeat points and can't self-detect redundancy

## Current Setup
- Council channel: #1495091563347448034
- Deliberators: Sable (nvidia-super:free, agitator), Locke (openai-gpt-oss-120b:free, skeptic), Maren (minimax-m2.5:free, arbiter)
- Moderator: Hart (TBD model, moderator/facilitator) — NEW, not yet deployed
- Each runs as separate systemd service: `hermes-gateway-{sable,locke,maren,hart}.service`
- Profiles at `~/.hermes/profiles/{sable,locke,maren,hart}/`
- `DISCORD_ALLOW_BOTS=mentions`, `DISCORD_REQUIRE_MENTION=true`
- Avatar for Hart: `~/.hermes/avatars/hart-v2.png` (Pollinations portrait — stern judge, dark robe with gold trim, clean hands)

## Fix Plan (Ordered by Priority)

### Step 1: History Injection (Critical — Do First)
Without this, agents are blind to prior conversation.

**Hermes issue #14853** has the patch. Apply `DISCORD_CHANNEL_HISTORY_SIZE=50` so each agent sees the last 50 messages before responding.

Alternative: Hermes issue #13054 proposes `history_backfill` and `history_backfill_limit` config options (not yet merged).

If patch isn't applied, the moderator agent must manually inject channel history into each prompt.

### Step 2: Add a Moderator Agent — "Hart"
Hart is a dedicated moderator agent, separate from the deliberators. Hart does NOT argue positions — only facilitates.

**Hart setup:**
- Name: Hart (H.L.A. Hart, legal philosopher — "heart" = impartial center)
- Model: Free tier, needs to be smart enough to summarize/detect repetition. Different provider from Sable/Locke/Maren for heterogeneity.
- Profile: `~/.hermes/profiles/hart/` with .env, SOUL.md, config
- Systemd: `hermes-gateway-hart.service`
- Avatar: `~/.hermes/avatars/hart-v1.png` (Pollinations portrait, stern judge in dark robes with gold trim)
- Discord bot token needed — create at discord.com/developers

**Hart's role:**
1. Posts the debate topic with clear scope and phase (EXPLORATION, DEBATE, or CONCLUSION)
2. @mentions council members with opening prompt
3. Collects responses **sequentially** (not parallel — tag agents one at a time)
4. After each agent responds:
   - Summarizes their argument in one sentence
   - Classifies as: CLAIM, CHALLENGE, EVIDENCE, or CONCESSION
   - Checks for novelty — if point was already made, notes the overlap
   - Updates consensus board (SETTLED if 2+ agree, OPEN if contested, NOVEL if new)
5. After each round, posts round summary with SETTLED/OPEN/NOVEL sections
6. Begins next round with: `SETTLED POINTS FROM PREVIOUS ROUND: [list]. NEW CONTRIBUTIONS ONLY — do not reargue settled points.`
7. Ends debate after: all open questions resolved, OR 3 rounds complete, OR no novel contributions in last round
8. Posts final synthesis with CONSENSUS, DISSENT, RECOMMENDATION, CONFIDENCE, MINORITY REPORT

**Hart SOUL.md is saved in conversation context (04/25 session). Key rules:**
- Never argue a position. Facilitate only.
- Kill loops aggressively: "Already settled. Move to an open question."
- Max 3 rounds per topic.
- Never use @ when referencing other agents by name (anti-cascade).
- Be brief — bullet points, not essays.

**Moderator prompt template (if not using Hart):**
```
After each agent responds, summarize: 1) Points of agreement, 2) Points of disagreement, 3) Novel contributions. Mark any point where ≥2 agents agree as "SETTLED". Begin next round prompt with: "SETTLED POINTS FROM PREVIOUS ROUND: [list]. NEW CONTRIBUTIONS ONLY - do not reargue settled points."
```

### Step 3: Structured Response Format
Require every council message to use:
```
[CLAIM/CHALLENGE/EVIDENCE/CONCESSION]: [point]
NOVELTY: 1-5
```
This forces self-awareness about repetition. If an agent scores own novelty < 3, moderator prompts them to revise.

### Step 4: SOUL.md Updates (All Three Agents)
Add to each council agent's SOUL.md:

```markdown
# Debate Rules
- Read the full channel history before responding — do not repeat arguments already made
- Tag every message as [CLAIM], [CHALLENGE], [EVIDENCE], or [CONCESSION]
- Rate your contribution's novelty (1-5) compared to what's already been said
- If your point was already made by another agent, acknowledge it and move to a different angle
- Never use @ when referencing other agents — refer to them by name only
- Respect SETTLED points — do not reargue what's been marked as consensus
- After 2 rounds on the same subtopic, you must either concede or introduce genuinely new evidence
```

### Step 5: Anti-Cascade Config (Already Partially Done)
- `DISCORD_ALLOW_BOTS=mentions` (NOT `all` — prevents cascade)
- `DISCORD_REQUIRE_MENTION=true`
- SOUL.md: "Never use @ when referencing other agents"
- `DISCORD_NO_THREAD_CHANNELS` = council channel ID (keep conversation inline)

## Research Sources

### Structured Debate (Proven Effective)
- **DCI research paper**: Deliberative structure beats unstructured debate by +0.95 quality points on non-routine tasks. Uses typed "epistemic acts" (claim, challenge, evidence, concession).
- **MAD (Multi-Agent Debate)**: Sequential protocol with judge mechanism. Agent heterogeneity (different models) improves GSM-8K from 82% to 91%.
- **Sparse communication topologies**: Reduces token cost 41% without accuracy loss.

### Hermes-Specific
- **Issue #14853**: History injection patch for multi-agent Discord
- **Issue #13054**: Native `history_backfill` config proposal
- **Issue #5876**: Community multi-agent council skill (solver-critic pattern)
- **Issue #344**: Multi-agent architecture vision (workflow DAGs, shared memory)

## Phase 2 (Optional Enhancements)
1. **Shared argument map**: A living markdown file tracking SETTLED vs OPEN questions, updated after each round
2. **Role rotation**: Every 2-3 rounds, rotate assigned perspectives to prevent entrenchment
3. **Loop detection**: String similarity check on agent responses; auto-advance phase if repetition detected
4. **Phase transitions**: Exploration → Consolidation → Convergence → Conclusion, declared by moderator

## Pitfalls
- Free-tier models (nvidia-super, openai-gpt-oss-120b, minimax-m2.5) struggle with complex multi-step instructions. Keep format rules simple.
- DCI research shows structured deliberation is 62× more expensive in tokens than single-agent. On free tiers with rate limits, keep debate rounds tight (2-3 max).
- Don't use DISCORD_ALLOW_BOTS=all — this causes cascade loops where every agent responds to every other agent infinitely.
- Sequential processing is slower than parallel but produces better results. Trade latency for quality.
