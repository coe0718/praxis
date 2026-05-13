# Codex VSCode Session Forensics

How to read Codex VSCode extension history directly from disk. Useful when Jeremy was working on something overnight and you need to pick up where he left off.

## When to Use

- Jeremy asks "what was I working on last night?"
- Need to continue work from a previous Codex session
- Want to see what Codex last said or what it was building

## Key Paths

All paths under `~/.codex/`:

| Path | Description |
|------|-------------|
| `sessions/` | Session rollout JSONL files, organized by `YYYY/MM/DD/` |
| `session_index.jsonl` | Quick index of all sessions (id, name, updated_at) |
| `state_5.sqlite` | SQLite DB with threads table (conversation metadata) |
| `logs_2.sqlite` | Internal logging (not conversation content) |
| `history.jsonl` | Older session history (not always populated for new sessions) |

## Session Index

The fastest way to find recent Codex sessions:

```bash
cat ~/.codex/session_index.jsonl | tail -5
```

Returns JSONL with fields: `id`, `thread_name`, `updated_at`.

## Thread Metadata

Get all recent Codex threads with their project paths:

```bash
sqlite3 ~/.codex/state_5.sqlite "
  SELECT id, title, cwd, source,
         datetime(created_at, 'unixepoch') as created,
         datetime(updated_at, 'unixepoch') as updated
  FROM threads
  ORDER BY updated_at DESC
  LIMIT 10;
"
```

Key columns:
- `title` — the session name/description (usually the user's first message)
- `cwd` — the working directory (tells you what project they were in)
- `updated_at` — last activity timestamp
- `source` — `vscode`, `cli`, or `{"subagent": {...}}` for subagent sessions

## Reading Session Content

Each session is stored as a JSONL file in `~/.codex/sessions/YYYY/MM/DD/`.

File naming pattern: `rollout-{timestamp}-{session_id}.jsonl`

To find a session file:
```bash
find ~/.codex/sessions -name "*{session_id_prefix}*"
```

To read the last messages (user query and agent response):
```bash
# Last 15KB of the session file - usually contains the final exchange
tail -c 15000 ~/.codex/sessions/2026/04/23/rollout-*.jsonl
```

Each line is a JSON object with `type` and `payload` fields. Relevant types:
- `session_meta` — session metadata (model, cwd, source, cli_version)
- `event_msg` with `agent_message` — Codex's final response text
- `event_msg` with `task_complete` — task completion marker with `last_agent_message`
- `response_item` with `type: message` — assistant messages
- `response_item` with `type: function_call` — tool calls made by Codex

## Parsing the Final Answer

The most useful info is usually in the last `event_msg` with `type: agent_message` or `type: task_complete`. Extract it with:

```bash
grep '"agent_message"' ~/.codex/sessions/2026/04/23/*.jsonl | tail -1 | python3 -c "
import sys, json
line = sys.stdin.read().strip()
if line:
    data = json.loads(line)
    print(data['payload']['message'])
"
```

## Common Patterns

### "Was working on PatchHive vs Praxis?"
Check the `cwd` column in the threads table — it tells you exactly which project directory Codex was running in.

### "How many API calls did they make?"
The last few lines before the final answer contain token count messages with `total_token_usage` data.

### "Was it a subagent?"
Check the `source` column in threads. If it's a JSON object with `subagent` field, it was a spawned sub-agent (not the main conversation).
