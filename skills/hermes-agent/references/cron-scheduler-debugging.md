# Cron Scheduler Debugging

When the Hermes cron scheduler stops firing jobs, the root cause is almost always `run_job()` in `cron/scheduler.py` hanging, which blocks the entire cron ticker.

## Architecture

- The gateway starts a cron ticker thread that calls `tick()` every 60 seconds.
- `tick()` acquires a file lock (`~/.hermes/cron/.tick.lock`), calls `get_due_jobs()`, advances next_run for each, then calls `run_job()` for each due job.
- `run_job()` creates an `AIAgent` instance and calls `agent.run_conversation(prompt)` in a `ThreadPoolExecutor`.
- The agent has a 600-second inactivity timeout, but if the agent hangs *during* the API call or init, it blocks the entire ticker until the timeout fires.

## Common Causes

### 1. Stale lock file
The gateway was killed (SIGKILL) and the lock file wasn't cleaned up. `flock` locks are FD-based and should be released on process death, but in practice the file can persist.
**Fix:** `rm -f ~/.hermes/cron/.tick.lock`

### 2. Model/provider auth failure
The cron job has `model: null, provider: null`, so it falls back to the main config's default model. If that model's API key is wrong or the endpoint is unreachable, the AIAgent hangs on init or API call.
**Diagnose:** Check what model the cron job resolves to:
- Main config's `model.default`
- Any env var `HERMES_MODEL`
- The cron job's explicit `model` field

**Fix:** Set explicit `model` and `provider` on the cron job, or fix the main config's credentials.

### 3. Agent init hangs on context loading
If the cron job has a `workdir` configured and that directory contains a large `AGENTS.md` or skill files, the agent may hang reading them. This is rare but can happen on slow filesystems.

## Debugging Procedure

### Step 1: Check if jobs are due
```bash
cd ~/.hermes/hermes-agent && source venv/bin/activate
python3 -c "
import sys
sys.path.insert(0, '.')
from cron.jobs import get_due_jobs
jobs = get_due_jobs()
print(f'{len(jobs)} due job(s)')
for j in jobs:
    print(f'  {j[\"id\"]} {j.get(\"name\",\"?\")} next={j.get(\"next_run_at\")}')
"
```

### Step 2: Check if run_job hangs
```bash
timeout 30 python3 -c "
import sys
sys.path.insert(0, '.')
from cron.jobs import get_due_jobs
from cron.scheduler import run_job
jobs = get_due_jobs()
if jobs:
    success, output, final, error = run_job(jobs[0])
    print(f'Success: {success}, error: {error}')
else:
    print('No due jobs to test')
"
```

If this hangs for 30+ seconds, `run_job()` is the problem.

### Step 3: Check AIAgent creation
If run_job hangs, narrow down whether it's the AIAgent init or the API call:
```bash
timeout 30 python3 -c "
import sys
sys.path.insert(0, '.')
from run_agent import AIAgent
# Try creating an agent with minimal config
agent = AIAgent(model='z-ai/glm-4.5-air:free', provider='openrouter', quiet_mode=True, skip_memory=True, skip_context_files=True)
print('AIAgent created successfully')
"
```

### Step 4: Workaround — system crontab
If the scheduler is broken, bypass it:
```bash
(crontab -l 2>/dev/null; echo "0 */6 * * * /usr/bin/python3 /home/coemedia/.hermes/scripts/echo_x_poster.py >> /home/coemedia/.hermes/scripts/echo_x_poster.log 2>&1") | crontab -
```

### Step 5: Restart the gateway
```bash
systemctl --user restart hermes-gateway.service
```
If it gets stuck on "deactivating", force kill:
```bash
systemctl --user kill hermes-gateway.service
sleep 2
systemctl --user start hermes-gateway.service
```
