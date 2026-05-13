# Codex Session Recovery

Recovering what Jeremy was working on in VS Code/Codex when he can't remember.

## Ground Truth Source

Codex stores all session state in SQLite at `~/.codex/state_5.sqlite`. The `threads` table is the authoritative source — it records every session's cwd, title, timestamps, and status.

## Queries

### Latest 10 threads (most reliable)
```sql
SELECT id, title, cwd, source, 
  datetime(created_at, 'unixepoch') as created,
  datetime(updated_at, 'unixepoch') as updated
FROM threads 
ORDER BY updated_at DESC 
LIMIT 10;
```

The `updated_at` column tells you the actual last activity time for each thread. The `title` tells you what the session was about. The `cwd` tells you which project directory was open.

### Find session file on disk
```bash
find ~/.codex/sessions -name "*SESSION_ID*" 2>/dev/null
```
Session files are stored under `~/.codex/sessions/YYYY/MM/DD/` based on creation date.

### Read recent conversation from session file
```bash
tail -c 15000 ~/.codex/sessions/YYYY/MM/DD/rollout-SESSION_ID.jsonl | python3 -c "
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line: continue
    try:
        obj = json.loads(line)
        payload = obj.get('payload', {})
        ptype = obj.get('type', '')
        if ptype == 'event_msg':
            msg = payload.get('message', '')
            if msg:
                print(f'[AGENT] {msg}')
        elif ptype == 'response_item':
            rtype = payload.get('type', '')
            if rtype == 'message':
                for block in payload.get('content', []):
                    if block.get('type') == 'output_text':
                        print(f'[FINAL] {block[\"text\"]}')
            elif rtype == 'function_call':
                name = payload.get('name', '')
                args = payload.get('arguments', '')
                if name in ('exec_command', 'read_file', 'write_file'):
                    print(f'[TOOL] {name} {args[:200]}')
    except:
        pass
"
```

### Key fields to look for in the JSONL
- `"type": "event_msg"` + `"payload": {"message": "..."}` — Codex's actual spoken response
- `"type": "response_item"` + `"payload": {"type": "message"}` — final answer
- `"type": "response_item"` + `"payload": {"type": "function_call"}` — what Codex did
- `"phase": "final_answer"` — marks the end of a turn
- `"type": "event_msg"` + `"payload": {"type": "task_complete"}` — session ended

## Session Index (fallback)

If `state_5.sqlite` is unavailable, the session index file provides thread names and IDs:
```bash
cat ~/.codex/session_index.jsonl
```
Format: `{"id":"...","thread_name":"...","updated_at":"..."}`

## Common Pitfalls

### File timestamps are NOT reliable
In this session, I saw Praxis files with newer timestamps and assumed Jeremy was working on Praxis. The Codex session DB showed he was actually in PatchHive (thread: "Add HiveCore provisioning flow", cwd: `/mnt/docker/code/patchhive`).

### State database may have empty result on error
If `sqlite3` returns exit code 1 with no output, the query might be malformed or the table structure has changed. Check the schema first:
```bash
sqlite3 ~/.codex/state_5.sqlite ".schema threads"
```

### Session files can be very large
The "Add HiveCore provisioning flow" session had 4,455 lines spanning 11 days. Reading the whole file is impractical — use `tail -c 15000` for the most recent activity.
