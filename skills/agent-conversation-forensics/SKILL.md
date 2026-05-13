---
name: agent-conversation-forensics
description: Investigate what an agent actually said or did by querying its SQLite state database, reconstructing full conversations, and cross-referencing platform logs
category: devops
---

# Agent Conversation Forensics

**Use when:** You need to definitively answer "what did agent X say/do?" — incident investigation, boundary violation audit, verifying agent actions, or reconstructing events from logs.

This skill provides a systematic approach to extracting and reconstructing agent conversations from Hermes state databases when platform message history is unavailable or incomplete.

## Context

Hermes agents persist conversation history in SQLite databases at `~/.hermes/profiles/<agent>/state.db`. These databases contain the complete message history, session metadata, and tool interactions for that agent. When platform-specific message logs are truncated, incomplete, or unavailable (Discord "messages file" errors, Telegram API limitations), the state database is the single source of truth.

## Prerequisites

- Agent must be running Hermes Gateway with SQLite state persistence enabled (default)
- Access to the agent's home directory (`~/.hermes/profiles/<agent>/`)
- The `sqlite3` CLI tool installed on the system

## The Process

### Step 1: Locate the State Database

Each agent's state is stored in its profile directory:

```
~/.hermes/profiles/<agent>/state.db
```

Common agents:
- `tuck/` — main assistant
- `drey/` — coding specialist
- `vex/` — reviewer
- `kai-voss/` — writer agent
- Council agents: `sable/`, `locke/`, `maren/`

Verify existence and size:

```bash
ls -lh ~/.hermes/profiles/<agent>/state.db
```

### Step 2: Understand the Database Schema

The database contains three primary tables:

- **sessions** — conversation sessions with metadata
  - `id` (TEXT, PRIMARY KEY) — session identifier like `20260424_151553_b2a9b22e`
  - `user_id` (TEXT) — platform user ID who initiated
  - `platform` (TEXT) — platform name (discord, telegram, etc.)
  - `title` (TEXT) — session title
  - `started_at` (REAL) — Unix timestamp
  - `ended_at` (REAL) — Unix timestamp (may be NULL for active sessions)
  - `message_count` (INTEGER)

- **messages** — individual message entries
  - `id` (INTEGER PRIMARY KEY)
  - `session_id` (TEXT) — references `sessions.id`
  - `role` (TEXT) — `user`, `assistant`, `tool`, `session_meta`
  - `content` (TEXT) — message body (may be JSON-encoded)
  - `timestamp` (REAL) — Unix timestamp
  - `tool_calls` (TEXT) — JSON array of tool invocations
  - `token_count` (INTEGER)

- **messages_fts** — full-text search index (for content searches)

Check schema:

```bash
sqlite3 ~/.hermes/profiles/<agent>/state.db ".schema messages"
```

### Step 3: Identify the Relevant Time Window

Convert human-readable time to Unix timestamp:

```bash
# April 24, 2026 3:16 PM Eastern (Daylight Time)
# Eastern is UTC-4 in April, so 15:16 ET = 19:16 UTC
date -d "2026-04-24 19:16 UTC" +%s
# Returns: 1777072560
```

Or compute in Python:

```python
import datetime, time
ts = int(time.mktime(datetime.datetime(2026, 4, 24, 19, 16).timetuple()))
```

**Tip:** Agent logs (`~/.hermes/logs/agent.log`) often include session IDs in brackets like `[20260424_151553_b2a9b22e]`. Use those to jump directly.

### Step 4: Find the Target Session(s)

**By time window:**

```bash
sqlite3 ~/.hermes/profiles/<agent>/state.db \
  "SELECT id, started_at, title, message_count FROM sessions \
   WHERE started_at >= 1777072560 AND started_at <= 1777076160 \
   ORDER BY started_at;"
```

**By session ID prefix (date-based):**

```bash
sqlite3 ~/.hermes/profiles/<agent>/state.db \
  "SELECT id, started_at, title FROM sessions WHERE id LIKE '20260424%';"
```

**Search log for session IDs first:**

```bash
grep "inbound message.*platform=discord" ~/.hermes/logs/agent.log | tail -50
```

The log line format includes timestamps, platform, user, and often the message text — use this to identify candidate session IDs before querying the database.

### Step 5: Extract Full Conversation

**Get all messages for a known session ID:**

```bash
SESSION_ID="20260424_151553_b2a9b22e"
sqlite3 ~/.hermes/profiles/<agent>/state.db -json \
  "SELECT role, content FROM messages WHERE session_id = '$SESSION_ID' ORDER BY id;"
```

The `content` field may be:
- **Plain text** — direct message body
- **JSON-encoded** — if the message includes formatting, replies, or structured data. Use Python to parse:

```python
import json, sqlite3
conn = sqlite3.connect('~/.hermes/profiles/drey/state.db')
rows = conn.execute("SELECT role, content FROM messages WHERE session_id = ?", [sid]).fetchall()
for role, content in rows:
    if content:
        try:
            parsed = json.loads(content)
            print(f"{role}: {parsed.get('content', content)}")
        except json.JSONDecodeError:
            print(f"{role}: {content}")
```

**Include timestamps and previews:**

```bash
sqlite3 ~/.hermes/profiles/<agent>/state.db -column \
  "SELECT strftime('%Y-%m-%d %H:%M:%S', timestamp, 'unixepoch') as time, \
          role, substr(content, 1, 200) as preview \
   FROM messages WHERE session_id = '$SESSION_ID' ORDER BY timestamp;"
```

### Step 6: Cross-Reference Platform Logs

The platform log (`~/.hermes/logs/agent.log`) shows inbound/outbound message routing:

```bash
# Find all Discord messages from agent
grep "platform=discord user=Drey" ~/.hermes/logs/agent.log | tail -20

# Find inbound messages from a specific user
grep "platform=discord user=coe0718" ~/.hermes/logs/agent.log | tail -20

# Search within a time window
sed -n '/2026-04-24 15:16/,/2026-04-24 15:20/p' ~/.hermes/logs/agent.log
```

Combine log timestamps with database queries to establish precise sequence of events.

### Step 7: Establish Chain of Custody

Document:
1. **Agent** — which agent's database was queried
2. **Session IDs** — all relevant sessions with timestamps
3. **Platform context** — Discord channel, Telegram chat, etc.
4. **Message sequence** — user → assistant → tool calls in order
5. **Tool invocations** — check `tool_calls` column for actions taken
6. **Timeline** — reconcile database timestamps with log timestamps

### Step 8: Handle Encoded Content

If `content` looks like escaped JSON (`\"content\":\"...\"`), it's a serialized message bundle. Parse with:

```python
import json
raw = '{"content":"[Replying to: \\"Hello\\"]\\\\n\\\\nHey there."}'
data = json.loads(raw)
print(data['content'])
```

The `[Replying to: "..."]` prefix indicates a threaded Discord reply — the quoted text is the previous message being referenced.

## Common Pitfalls

- **Missing `messages` table** — some agent profiles use different storage backends. Check `~/.hermes/profiles/<agent>/` for `state.db` vs `state.json` vs other formats.
- **Compressed content** — large tool outputs may be gzipped in the database. Use `-json` flag to let sqlite3 handle it, or query via Python's `zlib`.
- **Timestamp confusion** — agent `started_at` is Unix epoch; logs use human-readable local time. Convert carefully (agent uses system timezone; Eastern Time is UTC-4 during daylight saving).
- **Session chaining** — long-running agent conversations may span multiple sessions (timeout-based). Check `parent_session_id` in `sessions` table.
- **Platform-specific quirks** — Discord @mentions are redacted as `<@***>` in SQLite content but shown in plain text in logs. The actual user ID is stored separately in `user_id` columns.
- **Empty content** — `session_meta` role entries have `NULL` content. Skip them.
- **Truncated messages** — extremely long content may be truncated in the database preview. Use `-json` and parse the full `content` field.

## When to Use This Skill

- **Boundary violation investigation** — did agent X interfere with agent Y's domain?
- **Action verification** — did the agent actually modify file Z, or just discuss it?
- **Incident reconstruction** — build timeline of what happened before/after an error
- **Forensic audit** — answer "who said what to whom" with definitive evidence
- **Cross-platform correlation** — match Discord mentions to Telegram commands
- **Bug diagnosis** — determine if agent behavior matches intended logic
- **Compliance review** — verify agents haven't accessed unauthorized data

## When NOT to Use

- **Real-time monitoring** — use `hermes-gateway-debug` or log streaming instead
- **Mass data export** — write a custom Python script with sqlite3 module
- **Decryption needed** — if state is encrypted, this skill won't help (rare in Hermes)
- **Remote agent** — you need filesystem access to the host; cannot query remote databases over network

## Codex Session Forensics

Codex (OpenAI's coding agent) uses a **completely different** storage format from Hermes. If you need to find out what Codex was working on or what it said in a previous session:

### Storage Format

Codex stores sessions in two places:

1. **JSONL rollout files** — `~/.codex/sessions/<year>/<month>/<day>/rollout-<timestamp>-<session_id>.jsonl`
   - One JSON object per line, each with `timestamp`, `type`, and `payload` fields
   - Line types: `session_meta`, `event_msg` (agent_message, token_count, task_complete), `response_item` (function_call, function_call_output, reasoning, message)
   - Final response is an `event_msg` with `type: "agent_message"` and `phase: "final_answer"`
   - Rollout file encoding: the `cwd` field in `session_meta` shows which project was active

2. **state_5.sqlite** — `~/.codex/state_5.sqlite`
   - Main tables: `threads` (session metadata), `thread_dynamic_tools`, `thread_goals`, `agent_jobs`, `agent_job_items`
   - The `threads` table has: `id` (UUID), `title`, `cwd` (working directory), `source` (vscode/cli), `model_provider`, `created_at`/`updated_at` (Unix epoch), `agent_nickname`, `agent_role`, `first_user_message`, `model`, `git_sha`, `git_branch`, `git_origin_url`

### Step-by-Step: Find What Codex Was Working On

**Step 1: List recent Codex threads:**
```bash
sqlite3 ~/.codex/state_5.sqlite \
  "SELECT id, title, cwd, source, datetime(updated_at, 'unixepoch') as updated \
   FROM threads ORDER BY updated_at DESC LIMIT 10;"
```

**Step 2: Cross-reference with session index:**
```bash
cat ~/.codex/session_index.jsonl | tail -5
```

Each line has `id`, `thread_name`, and `updated_at`.

**Step 3: Locate the rollout file:**
The session `id` (UUID) is part of the rollout filename. Session created April 23 → file at `~/.codex/sessions/2026/04/23/rollout-*<uuid-prefix>*.jsonl`.

**Step 4: Read the most recent conversation turn:**
The file is one JSON object per line. The final few lines contain the last exchange. Tail the file:
```bash
tail -c 15000 ~/.codex/sessions/<year>/<month>/<day>/rollout-<uuid>.jsonl
```
Look for `event_msg` with `type: "agent_message"` and `phase: "final_answer"` — this is Codex's last response.

**Step 5: Find the user's question:**
Scroll backward from the final answer to find preceding `event_msg` entries with user query content and `response_item` entries showing Codex's reasoning, function calls, and tool outputs.

### Key Fields to Look For

- **`cwd`** — the project directory Codex was running in (e.g., `/mnt/docker/code/patchhive`, `~/projects/hackathon-creative`)
- **`title`** — the session name (e.g., "Add HiveCore provisioning flow", "Fix landing v3 parity")
- **`model_provider`** — `openai` for GPT-5, etc.
- **`source`** — `vscode` if Jeremy used the VS Code extension, `cli` if terminal, or a subagent object if spawned
- **`git_sha`, `git_branch`** — the git state at session start
- **`total_token_usage`** — in event_msg shows total tokens used (useful to see if the session completed or was interrupted)
- **`agent_nickname`** — if set, Codex spawned a sub-agent (e.g., "Curie" for explorer agent)
- **`task_complete`** event — marks the end of a session with `completed_at` timestamp

### Pitfalls

- **Rollout files are append-only** — the last few KB contain the most recent conversation turn
- **No message-level dedup** — the JSONL file has every tool call, every token count update, every reasoning chunk
- **Conversation content may be encrypted** — look for `encrypted_content` field; if present, the message text is not readable
- **Session spans multiple days** — a session created April 23 may have messages up to May 4; check `updated_at` in the `threads` table, not `created_at`
- **Sub-agent sessions** — Codex can spawn child agents (e.g., Curie). Those have their own rollout files under a different session ID
- **state_5.sqlite vs rollout files** — the SQLite DB has thread metadata (title, cwd, timing); the rollout JSONL has the actual conversation content. You need both.

### Example: Reconstructing Jeremy's Last Codex Session

From May 4, 2026:
1. `sqlite3 ~/.codex/state_5.sqlite "SELECT id, title, cwd, datetime(updated_at, 'unixepoch') FROM threads ORDER BY updated_at DESC LIMIT 3;"`
   → Found session `019dbd3e` titled "Add HiveCore provisioning flow", cwd: `/mnt/docker/code/patchhive`, updated May 4 at 02:39
2. Located rollout file at `~/.codex/sessions/2026/04/23/rollout-2026-04-23T22-07-49-019dbd3e-4bc1-7351-ad87-05c89195e800.jsonl`
3. Tailed last 15KB → Found Codex's final message: "Implemented and pushed all 4 pieces. Launcher product preflight details, per-product start/stop/restart endpoints, per-product logs endpoint, HiveCore setup UI controls plus scripts/smoke-launcher.sh."
4. Also found: user was working on PatchHive launcher, commits `78b26f4` and `12463ad` were pushed, CI passed

## Related Skills

- `devops/hermes-gateway-debug` — diagnose gateway freezes and crashes
- `devops/hermes-migration` — move agent installation to new host (includes state transfer)
- `multi-agent/discord-agent-interaction` — understand Discord agent behavior
- `software-development/systematic-debugging` — general debugging methodology

## Example: Investigating Drey's Involvement in Malik's Book

**Goal:** Determine whether Drey actually modified Malik's book chapters or merely discussed them.

**Process applied:**

1. Located Drey's state: `~/.hermes/profiles/drey/state.db` (6MB, active)
2. Checked sessions for April 24, 2026: found `20260424_151553_b2a9b22e` titled "Coding Assistance Inquiry"
3. Queried all messages in that session with JSON output
4. Parsed 9 messages total:
   - User (Tuck): "Drey — good to have you online. I'm deep in Malik's book proofread notes..."
   - Assistant (Drey): "Currently working on Praxis Phase 2A..."
   - User: dumped 5 critical continuity issues from Malik's book
   - Assistant: "Flag critical path-breaking ones first..."
   - User: provided full list of critical issues
   - Assistant: offered to structure errata file, asked "Want me to set up a structured errata file?"
5. **Finding:** Drey never committed any code changes, never accessed book files, never issued file-modifying tool calls. His last action was a clarifying question about tracking format.
6. **Conclusion:** Boundary violation was in **conversation initiation only** — Tuck involved Drey in Malik's domain. Drey's response was appropriate (stayed in his lane).

This pattern — agent database query + platform log cross-reference — can be reused for any incident requiring definitive proof of agent actions.

## Quick Reference Commands

```bash
# Find all sessions on a date
sqlite3 ~/.hermes/profiles/<agent>/state.db \
  "SELECT id, title, started_at FROM sessions WHERE id LIKE '20260424%';"

# Dump full conversation
sqlite3 ~/.hermes/profiles/<agent>/state.db -json \
  "SELECT role, content FROM messages WHERE session_id = 'SESSION_ID' ORDER BY id;"

# Search message content
sqlite3 ~/.hermes/profiles/<agent>/state.db \
  "SELECT session_id, role, substr(content,1,100) FROM messages \
   WHERE messages_fts MATCH 'keyword' ORDER BY timestamp DESC LIMIT 20;"

# Find tool invocations in a session
sqlite3 ~/.hermes/profiles/<agent>/state.db \
  "SELECT tool_name, content FROM messages WHERE session_id = 'SID' AND tool_calls IS NOT NULL;"

# Cross-check with logs (last 50 platform events for that agent)
grep "platform=discord user=<agent>" ~/.hermes/logs/agent.log | tail -50
```
