# Sandbox

> Per-channel filesystem isolation policy that restricts which tools and security levels are accessible to messages arriving from secondary channels.

## Overview

When Praxis receives messages from multiple channels — the primary operator Telegram chat, group chats, Discord servers, delegation links, or webhook endpoints — the Sandbox module ensures each channel operates within its defined boundaries. The primary operator surface (CLI, primary Telegram chat) always has full access. Every other channel can be assigned a `ChannelSandbox` policy that narrows the permitted tool surface.

This defense-in-depth mechanism prevents a compromised or untrusted channel from escalating to dangerous operations. For example, a team group chat might be restricted to internal read-only tools with forced approval, while a delegation link might only allow a specific subset of tools.

Policies are stored in `channel_sandbox.json` and evaluated on every tool invocation originating from a non-primary channel. When no policy exists for a channel, all tools are allowed (fail-open for backward compatibility). If the sandbox configuration file itself is unreadable, the system fails closed (blocks all tool execution).

## Architecture

### `ChannelSandbox` (struct)

A named restriction policy for a channel or delegation link.

| Field | Type | Default | Description |
|---|---|---|---|
| `label` | `String` | `""` | Human-readable label for display. |
| `allowed_tool_kinds` | `Vec<String>` | `[]` (all) | Tool kinds permitted (`"internal"`, `"shell"`, `"http"`). Empty = all kinds. |
| `denied_tool_name_patterns` | `Vec<String>` | `[]` | Glob patterns for tool names that are always blocked. |
| `max_security_level` | `Option<u8>` | `None` (no cap) | Maximum `required_level` allowed. |
| `force_approval` | `bool` | `false` | Force operator approval for every tool call, even if auto-approved globally. |

### `SandboxVerdict` (enum)

| Variant | Meaning |
|---|---|
| `Allow` | Tool execution permitted, proceed normally. |
| `Block(String)` | Tool blocked; contains a human-readable reason. |
| `RequireApproval` | Tool permitted but must go through the operator approval queue. |

### `ChannelSandboxStore` (struct)

A `HashMap<String, ChannelSandbox>` keyed by channel ID string. Provides load/save to JSON and CRUD operations.

### Preset constructors

- **`ChannelSandbox::strict(label)`** — Internal tools only, level ≤ 1, force approval.
- **`ChannelSandbox::read_only(label)`** — Internal tools only, blocks write patterns, level ≤ 2, no forced approval.

### Evaluation logic

`evaluate_tool(sandbox, tool_name, tool_kind, required_level)` checks in order:

1. **Security level cap** — Blocks if `required_level > max_security_level`.
2. **Denied name patterns** — Blocks if the tool name matches any glob pattern.
3. **Allowed tool kinds** — Blocks if the tool kind is not in the allowlist (empty allowlist = all kinds).
4. **Force approval** — If the tool passes all checks and `force_approval` is true, returns `RequireApproval`.

### Glob matching

Supports `*` wildcards: `risky-*` matches `risky-delete`, `*-write*` matches `praxis-data-write`. Simple prefix/suffix/infix matching without full glob syntax.

## Public API

```rust
// Evaluate a tool against a sandbox policy
evaluate_tool(
    sandbox: &ChannelSandbox,
    tool_name: &str,
    tool_kind: ToolKind,
    required_level: u8,
) -> SandboxVerdict

// Convenience: load store and evaluate in one call
check_channel_tool(
    paths: &PraxisPaths,
    channel_id: &str,
    tool_name: &str,
    tool_kind: ToolKind,
    required_level: u8,
) -> SandboxVerdict

// Store operations
ChannelSandboxStore::load(path: &Path) -> Result<Self>
ChannelSandboxStore::save(&self, path: &Path) -> Result<()>
ChannelSandboxStore::get(&self, channel_id: &str) -> Option<&ChannelSandbox>
ChannelSandboxStore::set(&mut self, channel_id: impl Into<String>, sandbox: ChannelSandbox)
ChannelSandboxStore::remove(&mut self, channel_id: &str) -> bool
ChannelSandboxStore::summary(&self) -> String
```

## Configuration

### `channel_sandbox.json` (located at `{data_dir}/channel_sandbox.json`)

```json
{
    "-1001234567890": {
        "label": "team-chat",
        "allowed_tool_kinds": ["internal"],
        "denied_tool_name_patterns": ["risky-*"],
        "max_security_level": 1,
        "force_approval": true
    },
    "-1009876543210": {
        "label": "monitoring-channel",
        "allowed_tool_kinds": ["internal", "http"],
        "denied_tool_name_patterns": [],
        "max_security_level": 2,
        "force_approval": false
    }
}
```

### Delegation links

When a delegation link carries a `sandbox` field, all inbound task requests arriving on that link are evaluated against the sandbox before the approval queue is consulted.

## Usage

```rust
use crate::sandbox::{ChannelSandbox, ChannelSandboxStore, check_channel_tool};

// Evaluate tool access for a channel
let verdict = check_channel_tool(
    &paths,
    "-1001234567890",
    "shell-exec",
    ToolKind::Shell,
    3,
);
match verdict {
    SandboxVerdict::Allow => { /* proceed */ },
    SandboxVerdict::Block(reason) => { /* reject */ },
    SandboxVerdict::RequireApproval => { /* queue for approval */ },
}

// Create a strict sandbox programmatically
let mut store = ChannelSandboxStore::load(&paths.sandbox_file)?;
store.set("-1001234567890", ChannelSandbox::strict("team-chat"));
store.save(&paths.sandbox_file)?;
```

## Data Files

| File | Purpose |
|---|---|
| `{data_dir}/channel_sandbox.json` | Per-channel sandbox policies (JSON format). |

## Dependencies

- **`tools/manifest`** — `ToolKind` for tool classification.
- **`paths`** — `PraxisPaths.sandbox_file` for the configuration file location.

## Source

`src/sandbox.rs`
