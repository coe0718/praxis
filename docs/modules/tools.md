# Tools

> Tool registry, execution engine, security policy, cooldown, redaction, container runtime detection, loop guard, and approval flow for the Praxis agent's action surface.

## Overview

The Tools module is the largest subsystem in Praxis, responsible for every action the agent can take beyond pure reasoning. It encompasses:

- **Tool manifests** — TOML files that define each tool's name, kind, security level, approval requirements, and execution parameters.
- **Registry** — Discovers, validates, and loads tool manifests from the filesystem.
- **Execution** — Dispatches tool invocations by name (built-in tools) or by kind (generic Shell/HTTP tools), with timeout enforcement, output caps, and secret injection.
- **Security policy** — Validates every tool request against security level, path allowlists, protected file circuit breakers, hardline blocklists, and payload size limits.
- **Cooldown** — Per-tool, per-path approval cooldown windows so the agent can repeat approved actions without fresh operator approval every time.
- **Redaction** — Strips common secret patterns from tool output when `security.redact_secrets` is enabled.
- **Loop guard** — Detects repeated tool invocation patterns (including multi-step loops) to prevent infinite tool-call cycles.
- **Tool use policy** — Fine-grained per-platform and per-channel access control via `tool_policy.toml`.
- **Container runtime** — Detects Docker or Podman availability for sandboxed code execution.

## Architecture

### Module structure

| Submodule | Purpose |
|---|---|
| `mod.rs` | Re-exports and public API surface. |
| `manifest.rs` | `ToolManifest`, `ToolKind` — tool definition types. |
| `registry.rs` | `ToolRegistry` trait, `FileToolRegistry` — manifest loading and validation. |
| `execute.rs` | `execute_request()` — dispatch and execution engine. |
| `policy.rs` | `SecurityPolicy` — request validation and circuit breakers. |
| `tool_policy.rs` | `ToolPolicy` — per-platform/channel tool access control. |
| `cooldown.rs` | `CooldownPolicy`, `CooldownStore` — approval cooldown windows. |
| `redact.rs` | `redact_output()` — secret pattern scrubbing. |
| `guard.rs` | `LoopGuard` — repeated invocation detection. |
| `container.rs` | `ContainerRuntime` — Docker/Podman detection. |
| `request.rs` | `ToolRequestPayload`, `build_payload()` — payload construction. |
| `clarify.rs` | Clarify-question tool for operator interaction. |
| `todo.rs` | In-session task list management. |
| `docs.rs` | `sync_capabilities()` — CAPABILITIES.md auto-generation. |

### Key types

**`ToolKind`** (enum) — `Internal`, `Shell`, `Http`. Determines how a tool is executed.

**`ToolManifest`** (struct) — Full definition of a tool:

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Unique tool identifier. |
| `description` | `String` | Human-readable description (used in prompts and MCP). |
| `kind` | `ToolKind` | Execution strategy. |
| `required_level` | `u8` | Security level 1–4. |
| `requires_approval` | `bool` | Whether operator approval is needed. |
| `rehearsal_required` | `bool` | Whether a dry-run rehearsal is required before execution. |
| `allowed_paths` | `Vec<String>` | File paths the tool may write to. |
| `allowed_read_paths` | `Vec<String>` | File paths the tool may read from. |
| `path` | `Option<String>` | Executable path for Shell tools. |
| `args` | `Vec<String>` | Fixed arguments prepended before request args. |
| `timeout_secs` | `Option<u64>` | Wall-clock timeout (default 30s). |
| `endpoint` | `Option<String>` | URL template for HTTP tools (supports `{param}` placeholders). |
| `method` | `Option<String>` | HTTP method (GET/POST/PUT/DELETE/PATCH). |
| `headers` | `Vec<String>` | Extra HTTP headers as `"Key: Value"` strings. |
| `body` | `Option<String>` | JSON body template with `{param}` substitution. |
| `allowed_vault_keys` | `Option<Vec<String>>` | Restrict which vault secrets are injected as env vars. |
| `allowed_oauth_providers` | `Option<Vec<String>>` | Restrict which OAuth tokens are injected. |

**`ToolExecutionResult`** — Contains a `summary` string with the tool's output.

**`SecurityPolicy`** — Stateless validator that checks: hardline blocklist, `[IMPORTANT:]` injection, security level, write path limits, protected file limits, path traversal, and payload size.

**`LoopGuard`** — Tracks a rolling window of 12 tool invocation hashes and detects repeated patterns of length 1–3 exceeding the limit (default 3).

**`CooldownPolicy`** / **`CooldownStore`** — Configurable per-tool, per-path cooldown windows. Persisted in `tool_cooldowns.json`.

### Built-in tools

| Tool Name | Kind | Level | Approval | Description |
|---|---|---|---|---|
| `internal-maintenance` | Internal | 1 | No | Safe bookkeeping, no external side effects. |
| `file-read` | Internal | 1 | No | Read files from data_dir or `allowed_read_paths`. |
| `todo` | Internal | 1 | No | In-session task list (create/update/complete/cancel/list). |
| `memory` | Internal | 1 | No | Persistent operator memory across sessions. |
| `clarify` | Internal | 1 | No | Ask the operator a clarifying question. |
| `vision` | Internal | 1 | No | AI image analysis. |
| `voice` | Internal | 1 | No | Speech-to-text and text-to-speech. |
| `cron` | Internal | 2 | No | Schedule recurring/one-shot tasks. |
| `image` | Internal | 2 | No | DALL-E image generation. |
| `praxis-data-write` | Shell | 2 | Yes + rehearsal | Controlled writes to data_dir markdown files. |
| `git-query` | Shell | 2 | Yes | Read-only git commands via `/usr/bin/git`. |
| `web-fetch` | Http | 2 | Yes | HTTP GET requests with SSRF protection. |
| `code_exec` | Internal | 2 | Yes | Sandboxed code execution. |
| `browser` | Internal | 2 | Yes | Headless browser automation. |
| `shell-exec` | Shell | 3 | Yes + rehearsal | Arbitrary shell commands via `/bin/bash -c`. |

## Public API

```rust
// Registry
trait ToolRegistry {
    fn ensure_foundation(&self, paths: &PraxisPaths) -> Result<()>;
    fn validate(&self, paths: &PraxisPaths) -> Result<()>;
    fn list(&self, paths: &PraxisPaths) -> Result<Vec<ToolManifest>>;
    fn get(&self, paths: &PraxisPaths, name: &str) -> Result<Option<ToolManifest>>;
    fn register(&self, paths: &PraxisPaths, manifest: &ToolManifest) -> Result<PathBuf>;
    fn summary(&self, paths: &PraxisPaths) -> Result<String>;
}

// Execution
execute_request(paths, manifest, request, redact_secrets) -> Result<ToolExecutionResult>

// Security
SecurityPolicy.validate_request(config, paths, manifest, request) -> Result<()>

// Loop guard
LoopGuard.record(&self, state, invocation_key, limit) -> GuardDecision

// Cooldown
CooldownStore::load(path) / .save(path)
CooldownStore.record_approval(tool_name, path, now)
CooldownStore.is_within_cooldown(policies, tool_name, path, now) -> bool

// Redaction
redact_output(text: &str) -> String

// Tool use policy
ToolPolicy::load_or_default(path) -> Result<ToolPolicy>
ToolPolicy.resolve_tools(channel_id, platform) -> ResolvedToolPolicy

// Payload
build_payload(manifest, append_text, params) -> Result<Option<String>>
```

## Configuration

### `praxis.toml` fields

```toml
[security]
level = 2                    # Max tool level allowed (1-4)
redact_secrets = false       # Scrub secret patterns from tool output
hardline_blocklist = [       # Unrecoverable command blocklist
    "rm -rf /",
    "mkfs",
    "dd if=/dev/zero",
]

[[tool_cooldown]]
tool_name    = "praxis-data-write"
path_pattern = "JOURNAL.md"  # Optional glob; omit for all paths
window_secs  = 1800          # 30 minutes
```

### Tool manifest files (`{data_dir}/tools/*.toml`)

```toml
name = "web-fetch"
description = "Fetch a URL via HTTP GET."
kind = "http"
required_level = 2
requires_approval = true
endpoint = "{url}"
method = "GET"
timeout_secs = 30
```

### Tool use policy (`tool_policy.toml`)

```toml
default_allow_all = false

[toolsets.readonly]
tools = ["file-read", "git-query"]
description = "Read-only tools"

[platforms.telegram]
toolset = "readonly"
require_mention = false
allow_all = false

[channels."-1001234567890"]
toolset = "readonly"
```

### Data files

| File | Purpose |
|---|---|
| `{data_dir}/tools/*.toml` | Tool manifest definitions. |
| `{data_dir}/tool_cooldowns.json` | Persisted approval cooldown timestamps. |
| `{data_dir}/tool_policy.toml` | Per-platform/channel tool access control. |

## Usage

### CLI commands (`praxis tools`)

```bash
# List all registered tools
praxis tools list

# Register a new tool
praxis tools register \
    --name "my-tool" \
    --description "Does something useful" \
    --kind shell \
    --level 2 \
    --approval

# Request tool execution
praxis tools request \
    --name "file-read" \
    --param "path=GOALS.md"
```

### Shell tool execution flow

1. Agent produces a tool request during the Decide/Act phase.
2. `SecurityPolicy.validate_request()` checks blocklist, security level, path constraints, and payload size.
3. If `requires_approval`, the request enters the approval queue.
4. Operator approves via `praxis approvals`.
5. `execute_request()` dispatches by tool name, then by kind.
6. Shell tools spawn with vault secrets and OAuth tokens injected as environment variables.
7. HTTP tools substitute `{param}` and `$VAULT{name}` placeholders, enforce SSRF protection.
8. Output is capped at 1 MB, trimmed to 500 characters in summaries.
9. If `redact_secrets` is enabled, output passes through `redact_output()`.
10. `LoopGuard` checks for repeated patterns; `CooldownStore` checks for active cooldowns.

## Dependencies

- **`vault`** — Secret injection into Shell/HTTP tool environments.
- **`oauth`** — OAuth token injection for tool execution.
- **`paths`** — File paths for tools directory, cooldown store.
- **`config`** — Security settings, cooldown policies.
- **`storage`** — `StoredApprovalRequest` for execution context.
- **`state`** — `SessionState` for loop guard history.
- **`sandbox`** — Channel-level tool access filtering.

## Source

`src/tools/` — `mod.rs`, `manifest.rs`, `registry.rs`, `execute.rs`, `policy.rs`, `tool_policy.rs`, `cooldown.rs`, `redact.rs`, `guard.rs`, `container.rs`, `request.rs`, `clarify.rs`, `todo.rs`, `docs.rs`, and more.
