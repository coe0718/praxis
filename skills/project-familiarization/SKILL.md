---
name: project-familiarization
description: Approach for quickly getting up to speed on a user's project or recovering context of what they were working on
trigger: |
  - User shares project links (website, GitHub, etc.) and asks agent to familiarize
  - User asks "what was I working on?" or doesn't remember their last session
  - Need to recover context of recent Codex/VS Code activity
author: Hermes Agent
version: 1.1
---

# Project Familiarization Skill

Use when the user needs you to get up to speed on a project or recover context of what they were working on.

## Mode A: Familiarization from shared links

When a user shares links to their project and asks you to familiarize yourself with it:

1. **Visit provided links systematically**
   - If website URL is provided: Navigate to it and take a snapshot
   - If GitHub/repo URL is provided: Navigate to it and take a snapshot
   - If other relevant links are provided (docs, demo, etc.): Visit as appropriate

2. **Extract key information**
   - From website: Project description, current status/metrics, live demo/stream links
   - From repository: 
     - Primary language/tech stack (look at src/ directory, package files)
     - Recent activity (latest commit, commit frequency)
     - Project structure (key directories like src/, skills/, docs/, etc.)
     - Any visible goals/TODOs/issues
     - Build/run instructions if visible in README

3. **Focus on what user emphasized**
   - Pay special attention to anything the user specifically pointed out or called their "favorite"
   - Note any current goals or metrics they highlighted

4. **Save to memory**
   - Save the project links
   - Save key details extracted (description, tech stack, current status/goals)
   - Save any specific highlights the user mentioned

5. **Offer to help further**
   - After familiarizing, ask if they want help with specific aspects
   - Suggest how you could assist based on what you learned

## Mode B: Recover session context (user doesn't remember what they were doing)

When the user says "I don't remember if we finished X" or "what was I working on?", do NOT guess based on file timestamps alone — use the actual artifact trail:

1. **Check Codex session state database FIRST**
   ```bash
   # Most recent threads, ordered by updated_at
   sqlite3 ~/.codex/state_5.sqlite "SELECT id, title, cwd, datetime(updated_at, 'unixepoch') as updated FROM threads ORDER BY updated_at DESC LIMIT 10;"
   ```
   This tells you exactly which project cwd and session title were last active.

2. **Read the actual Codex conversation tail**
   ```bash
   # Find the session file for the most relevant thread
   find ~/.codex/sessions -name "*SESSION_ID*" 2>/dev/null
   # Read the last ~15KB of the file for the most recent exchanges
   tail -c 15000 ~/.codex/sessions/2026/MM/DD/rollout-SESSION_ID.jsonl
   ```
   Parse the JSONL for `agent_message` and user message content to see Codex's final response.

3. **Check git log for recent commits**
   ```bash
   git log --oneline -5
   ```
   Cross-reference commit messages with Codex's final response.

4. **Check file timestamps as a secondary signal** (not primary — they can mislead)
   ```bash
   find /path/to/repo -name "*.rs" -o -name "*.ts" -o -name "*.tsx" | while read f; do
     stat -c '%Y %n' "$f" 2>/dev/null
   done | sort -rn | head -10
   ```

5. **Pitfall: Don't guess**
   - File timestamps can point to the wrong project (e.g., CI writes or dependency updates can be newer than actual work)
   - Codex session DB is the ground truth — it records the cwd, title, and update time of every thread
   - Always cross-reference Codex session data with git history before concluding

6. **Synthesize and report** — give a concise summary of what was done, what state it's in, and what needs attention

## Notes

- This skill is for initial familiarization and context recovery — not for deep code analysis or contribution
- Adjust depth based on what user seems to need (just awareness vs. planning to work on it)
- Always save findings to memory so they persist across sessions
- If user wants actual work done on the project, consider other skills like "plan" or "systematic-debugging"