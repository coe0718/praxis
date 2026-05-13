---
name: hermes-hindsight-troubleshooting
category: devops
description: Troubleshooting and fixing Hindsight memory plugin issues in Hermes Agent
---

# Hermes Hindsight Troubleshooting

Guidance for diagnosing and fixing common Hindsight memory plugin issues in Hermes Agent, particularly the missing `hindsight_embed` module and daemon startup failures.

## Common Issues

### 0. Diagnostic Flow — "Cannot connect to host 127.0.0.1:9177"
**Symptom:** Tool calls (`hindsight_retain`, `hindsight_recall`, etc.) fail with:
```
Cannot connect to host 127.0.0.1:9177
```

**Diagnostic sequence:**
```bash
# 1. Check if daemon port is listening
ss -tlnp | grep 9177

# 2. Check config mode
cat ~/.hermes/hindsight/config.json

# 3. Check daemon startup log for failures
tail -30 ~/.hermes/logs/hindsight-embed.log

# 4. Try manual start (this bypasses fragile auto-start)
cd ~/.hermes/hermes-agent && source venv/bin/activate && hindsight-embed -p hermes daemon start

# 5. Verify health after start
curl -s http://127.0.0.1:9177/health
```

**Daemon lifecycle:** In `local_embedded` mode, the daemon auto-starts on first tool use and auto-stops after **5 minutes of inactivity**. The plugin's auto-restart mechanism is fragile — when it fails, the CLI `daemon start` command is the reliable fallback. The daemon is **not** a systemd service by default.

### 1. Missing `hindsight_embed` Module
**Symptom:** Repeating daemon startup failures in `~/.hermes/logs/hindsight-embed.log`:
```
=== Daemon startup failed: No module named 'hindsight_embed' ===
```

**Root Cause:** The Hindsight plugin is configured for `local_embedded` mode but the required `hindsight-all` package is missing.

### 2. `websockets` Dependency Conflict (google-genai transitive)
**Symptom:** Daemon startup log shows repeated traceback ending with:
```
ImportError: cannot import name 'BytesLike' from 'websockets.typing'
```
Followed by either a failed auto-start or a successful manual start on retry. The full traceback chain: `hindsight → hindsight_api → google.genai → websockets.asyncio → websockets.protocol → websockets.typing`.

**Root Cause:** The `google.genai` package (installed as a transitive dependency of `hindsight-all`) pulls in `websockets` at a version where `BytesLike` was moved/renamed. The import fails during daemon init when the plugin tries to start the embedded daemon, but the `hindsight-embed` CLI binary bypasses this path.

**Fix — Manual start bypass:**
```bash
cd ~/.hermes/hermes-agent && source venv/bin/activate && hindsight-embed -p hermes daemon start
```
This reliably starts the daemon even when auto-start fails. If the CLI also fails, try pinning websockets:
```bash
pip install websockets==13.1
```
Then retry the manual daemon start.

**Long-term fix:** Either pin `websockets` in the venv, or switch to Hindsight Cloud mode to avoid the local dependency chain entirely.

### 3. Silent Failures in Local Mode
**Symptom:** Plugin silently fails in `local_embedded` or legacy `local` mode with WARNING logs but no functional memory.

**Root Cause:** Legacy configuration using deprecated `"mode": "local"` which was silently remapped to `local_embedded` but without proper dependencies.

## Solutions

### Option 1: Setup Wizard (Recommended)
Use the official setup wizard to automatically configure Hindsight:
```bash
hermes memory setup  # select "hindsight"
```
- Automatically installs correct dependencies
- Handles configuration properly
- Best for most users

### Option 2: Manual Package Installation
Install the required Python package for local embedded mode:
```bash
pip install hindsight-all
```
- Required for `local_embedded` mode
- May need to restart Hermes afterward

### Option 3: Switch to Cloud Mode
Configure cloud-based Hindsight service:
- Set memory provider to hindsight via `hermes config set memory.provider hindsight`
- Obtain API key from Hindsight cloud service
- Configure cloud API endpoint in environment

### Option 4: Local External Mode
For existing self-hosted Hindsight instances:
- Configure to point to existing Hindsight instance URL
- Use mode: `local_external` in configuration
- Optional API key if required

## Debugging Steps

### Check Plugin Registration
Verify the Hindsight plugin is properly registered in the system.

### Verify Health (Local Mode)
Check if local Hindsight daemon is running and healthy.

### Check Memory Status
Verify memory system status using Hermes commands.

### Disable Built-in Memory (if conflicting)
Prevent conflicts with default Hermes memory system.

## Configuration File Location
- **Config:** `~/.hermes/hindsight/config.json`
- **Logs:** `~/.hermes/logs/hindsight-embed.log`
- **Requirements:** `hindsight-client >= 0.4.22`

## Per-Agent Memory Bank Isolation

If multiple agents share memory banks, they can overwrite each other's context. To give each profile agent their own isolated memory bank:

### The Fix — `bank_id_template`

In `~/.hermes/hindsight/config.json`, add a dynamic template:

```json
{
  "bank_id": "hermes",
  "bank_id_template": "{profile}"
}
```

**How it works:** The `{profile}` placeholder resolves to each agent's profile name at runtime. Agents running under profiles (`drey`, `vex`, `echo`, etc.) each get their own bank. The instance with no profile (e.g., main Tuck gateway) falls back to `bank_id: "hermes"`.

### Applying the Change

After editing the config, restart all agent gateways:

```bash
for agent in drey echo herald kai-voss locke maren sable scout vex; do
  systemctl --user restart hermes-gateway-$agent
done
```

### Result

| Agent | Memory Bank |
|-------|-------------|
| Drey | `drey` |
| Vex | `vex` |
| Scout | `scout` |
| Echo | `echo` |
| Herald | `herald` |
| Kai Voss | `kai-voss` |
| Sable | `sable` |
| Locke | `locke` |
| Maren | `maren` |
| Tuck (no profile) | `hermes` (fallback) |

## Operational Modes Comparison

| Mode | Dependencies | Complexity | Use Case |
|------|-------------|------------|----------|
| **Cloud** | API key only | Lowest | Easy setup, no maintenance |
| **Local Embedded** | hindsight-all, LLM key | Medium | Self-contained, local processing |
| **Local External** | Existing Hindsight instance | Medium | Connect to deployed instances |

### Ultimate Fix — Switch to Holographic

If Hindsight keeps failing despite troubleshooting (daemon crashes, dependency conflicts, idle timeouts), the root cause is Hindsight's daemon-based architecture. **Holographic** is a drop-in replacement that's:
- Pure SQLite — zero daemons, zero idle timeouts
- stdlib only — no dependency conflicts
- Auto-isolates per agent profile
- Same tool interface (`hindsight_retain`, `hindsight_recall`, etc.)

See the `hermes-memory-provider-switch` skill for the full evaluation and migration guide.

## Prevention Tips

1. **Use Setup Wizard:** Always prefer `hermes memory setup` for initial configuration
2. **Check Dependencies:** Verify `hindsight-all` is installed for local modes
3. **Monitor Logs:** Regularly check `~/.hermes/logs/hindsight-embed.log` for issues
4. **Version Compatibility:** Ensure you have `hindsight-client >= 0.4.22`
5. **Auto-Update:** Plugin automatically upgrades client on session start if outdated

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `HINDSIGHT_API_KEY` | Cloud API access |
| `HINDSIGHT_LLM_API_KEY` | Local LLM key |
| `HINDSIGHT_API_URL` | Custom endpoint |
| `HINDSIGHT_MODE` | Override mode |

### 4. pgvector GLIBC Incompatibility (pg0 embedded Postgres)
**Symptom:** Database migration fails with:
```
psycopg2.errors.UndefinedFile: could not load library ".../vector.so": version `GLIBC_2.38' not found
RuntimeError: Database migration failed
```

**Root Cause:** The pgvector extension bundled with pg0's Postgres installation was compiled against a newer GLIBC than the host system. Common on Ubuntu 22.04 (GLIBC 2.35) when pg0 was built on a newer system.

**Diagnosis:**
```bash
ldd --version  # Check system GLIBC (e.g., 2.35)
ldd ~/.pg0/installation/*/lib/vector.so | grep GLIBC  # Check what vector.so needs
tail -50 ~/.hindsight/profiles/hermes.log | grep -i "vector\|GLIBC\|migration"
```

**Fix — Rebuild pgvector from source:**
```bash
# 1. Clone pgvector matching the installed version
cd /tmp && mkdir pgvector-build && cd pgvector-build
git clone --branch v0.8.1 --depth 1 https://github.com/pgvector/pgvector.git
cd pgvector

# 2. Build using pg0's pg_config (links against system GLIBC)
PATH="$HOME/.pg0/installation/18.1.0/bin:$PATH" \
  PG_CONFIG=$HOME/.pg0/installation/18.1.0/bin/pg_config \
  make

# 3. Verify no GLIBC 2.38 dependency
ldd vector.so | grep GLIBC  # Should show only system version

# 4. Backup old, install new
cp ~/.pg0/installation/18.1.0/lib/vector.so ~/.pg0/installation/18.1.0/lib/vector.so.broken
cp vector.so ~/.pg0/installation/18.1.0/lib/vector.so

# 5. Kill stale postgres, let hindsight restart it
kill $(pgrep -f "hindsight-embed-hermes")

# 6. Verify — check logs for successful migration
tail -30 ~/.hindsight/profiles/hermes.log | grep -i "migration completed"
curl -s http://localhost:9177/health  # Should return {"status":"healthy"}
```

**Prerequisites:** `build-essential`, `gcc`, `make`, and pg0's Postgres dev headers (included in pg0 installation at `--includedir-server`).

**After fixing:** The Hermes gateway may still hold a stale hindsight connection. Restart it to reconnect:
```bash
systemctl --user restart hermes-gateway
```

## Additional Resources

- [Hindsight Integration Documentation](https://hindsight.vectorize.io/sdks/integrations/hermes)
- [Hermes Hindsight Plugin README](https://github.com/NousResearch/hermes-agent/blob/main/plugins/memory/hindsight/README.md)
- [Hindsight Cloud Service](https://ui.hindsight.vectorize.io/connect)