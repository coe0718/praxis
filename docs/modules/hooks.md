# Hooks

> Shell-script lifecycle hooks — interceptors, observers, and approval automation bound to runtime events.

## Overview

The hooks module lets operators customize Praxis behavior without modifying source code. Hooks are shell scripts declared in `hooks.toml` that run in response to specific runtime events. Three kinds of hooks are supported:

- **Observer** — fire-and-forget side effects (notifications, logging, metrics). Non-blocking; the agent never waits for observers.
- **Interceptor** — blocking gatekeepers. A non-zero exit aborts the event. Used for safety checks, maintenance windows, or conditional feature toggling.
- **Approval** — automated approval decisions. The script prints `approve`, `reject <reason>`, or `defer` to stdout. The first decisive verdict wins.

Hooks receive context as environment variables (event name, data directory, phase, tool name, session ID, outcome) and are subject to security constraints: scripts must use absolute paths, symlinks are rejected, and each hook has a configurable timeout (default 10s).

## Architecture

### Key Types

| Type | Description |
|---|---|
| `HookEntry` | A single hook definition: event pattern, kind, script path, filter glob, timeout |
| `HookKind` | Enum: `Observer`, `Interceptor`, `Approval` |
| `HookConfig` | Container for a list of `HookEntry` items (maps to `hooks.toml`) |
| `HookContext` | Environment variables passed to every hook script |
| `HookRunner` | Loads hooks and executes matching entries for a given event |
| `ApprovalVerdict` | Result of an approval hook: `Approve`, `Reject(String)`, or `Defer` |

### Hook Lifecycle

**Observer:**
```
event → find matching hooks → spawn each as background process → don't wait
```

**Interceptor:**
```
event → find matching hooks → run each synchronously → if any exits non-zero → abort event
```

**Approval:**
```
approval.before → find matching approval hooks → run first → read stdout
  → "approve"  → auto-approve, skip operator queue
  → "reject X" → auto-reject with reason
  → "defer"    → continue to next hook or fall through to normal approval
```

### Event Matching

Events support simple glob patterns:
- `session.end` — exact match
- `phase.*` — matches `phase.orient.start`, `phase.reflect.end`, etc.
- `tool.*` — matches all tool events

A secondary `filter` field matches tool names or phase names.

### Built-in Hook Points

| Event | When |
|---|---|
| `session.start` | Before the first phase |
| `session.end` | After Reflect completes and state is persisted |
| `phase.<name>.start` | Before each phase begins |
| `phase.<name>.end` | After each phase completes |
| `tool.before` | Before tool execution (interceptor can block) |
| `tool.after` | After tool execution (observer) |
| `approval.before` | Before operator approval decision (approval hook) |

## Public API

### Loading Hooks

```rust
let runner = HookRunner::load(&paths.hooks_file)?;
let runner = HookRunner::from_paths(&paths);  // unwraps to default
```

### Firing Hooks

```rust
// Observer — non-blocking
runner.fire_observer("session.end", &ctx, "*");

// Interceptor — blocking, may abort
runner.fire_interceptor("tool.before", &ctx, "shell-exec")?;

// Approval — returns verdict
let verdict = runner.fire_approval_hooks("shell-exec", &ctx, Some(payload_json));
```

### Building Context

```rust
let ctx = HookContext::new("session.end", paths.data_dir.clone())
    .with_session(42)
    .with_phase("reflect")
    .with_outcome("goal_completed");
```

### CLI Helpers

```rust
install_hook(&paths, HookEntry { ... })?;  // Add hook to hooks.toml
remove_hook(&paths, script_path)?;          // Remove hooks by script path
```

## Configuration

### `hooks.toml` Format

```toml
[[hooks]]
event   = "session.end"
kind    = "observer"
script  = "/home/user/notify.sh"

[[hooks]]
event   = "phase.act.start"
kind    = "interceptor"
script  = "/home/user/gate.sh"
filter  = "*"
timeout_secs = 10

[[hooks]]
event   = "approval.before"
kind    = "approval"
script  = "/home/user/auto-approve.sh"
filter  = "safe-read-*"    # glob on tool_name
timeout_secs = 5
```

### Environment Variables (passed to every hook)

| Variable | Value |
|---|---|
| `PRAXIS_EVENT` | Event name (e.g., `session.end`) |
| `PRAXIS_DATA_DIR` | Absolute path to the data directory |
| `PRAXIS_SESSION_ID` | Current session DB id (if known) |
| `PRAXIS_PHASE` | Current phase name (if applicable) |
| `PRAXIS_TOOL_NAME` | Tool name (tool hooks only) |
| `PRAXIS_TOOL_REQUEST_ID` | Approval request id (tool hooks) |
| `PRAXIS_OUTCOME` | Session outcome (`session.end` only) |
| `PRAXIS_APPROVAL_JSON` | Request payload JSON (approval hooks only) |

### Security Rules

- Scripts **must** use absolute paths — relative paths are skipped with a warning
- Symlinked scripts are rejected for security
- Each hook has a configurable timeout (default 10 seconds); timed-out processes are killed

### CLI

```bash
praxis hooks list              # show registered hooks
praxis hooks install ...       # add a hook entry
praxis hooks remove <script>   # remove hooks by script path
```

## Data Files

| File | Purpose |
|---|---|
| `hooks.toml` | Hook definitions. Absent = no hooks. |

## Dependencies

- **paths** — `PraxisPaths` for `hooks_file` location

## Source

`src/hooks.rs`
