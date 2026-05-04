# Plugins

> TOML plugin manifests and runtime hook wiring for extending Praxis with custom tools, messaging platforms, and lifecycle hooks.

## Overview

The Plugins module enables operators to extend Praxis without modifying the core codebase. Plugins are self-contained directories, each with a `plugin.toml` manifest that declares tools, messaging platform integrations, and lifecycle hooks. At startup, the `PluginRegistry` scans the plugins directory, loads valid manifests, and makes the registered capabilities available to the runtime.

Plugins can:

- **Register new tools** — Shell commands that become available to the agent through the tool system.
- **Register messaging platforms** — Enable inbound/outbound messaging on platforms not built into Praxis.
- **Hook into lifecycle events** — Run shell scripts before prompts (`pre_prompt`), after responses (`post_response`), or on errors (`on_error`).
- **Block tool execution** — Pattern-match tool commands and block matches before execution.
- **Transform tool output** — Rewrite tool results before they reach the agent.

The plugin system is designed for safe extensibility: each plugin is isolated in its own directory, invalid manifests are logged but don't prevent other plugins from loading, and the block/rewrite hooks provide guardrails against dangerous operations.

## Architecture

### `Plugin` (struct)

A loaded plugin with all its capabilities.

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Plugin identifier. |
| `version` | `String` | Semantic version. |
| `hooks` | `PluginHooks` | Lifecycle hook definitions. |
| `tools` | `Vec<ToolRegistration>` | Tools this plugin provides. |
| `platforms` | `Vec<PlatformRegistration>` | Messaging platforms this plugin enables. |

### `PluginHooks` (struct)

| Field | Type | Description |
|---|---|---|
| `pre_prompt` | `Option<PathBuf>` | Script to run before each LLM prompt. |
| `post_response` | `Option<PathBuf>` | Script to run after each LLM response. |
| `on_error` | `Option<PathBuf>` | Script to run on errors. |
| `tool_block` | `Vec<String>` | Command patterns that block tool execution. |
| `tool_rewrite` | `Vec<String>` | Commands that transform tool output. |

### `ToolRegistration` (struct)

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Tool name for the registry. |
| `command` | `PathBuf` | Shell command to execute. |
| `required_level` | `u8` | Security level (default 0). |
| `requires_approval` | `bool` | Whether approval is needed (default false). |

### `PlatformRegistration` (struct)

| Field | Type | Description |
|---|---|---|
| `name` | `String` | Platform identifier (e.g. `"teams"`). |
| `enabled` | `bool` | Whether the platform is active (default true). |

### `PluginRegistry` (struct)

Holds all loaded plugins and provides lookup, blocking checks, and tool aggregation. Initialized with `PraxisPaths` for resolving the plugins directory.

## Public API

```rust
// Plugin loading
Plugin::from_dir(dir: &Path) -> Result<Plugin>

// Registry
PluginRegistry::new(paths: &PraxisPaths) -> Self
PluginRegistry::load_all(&mut self) -> Result<()>
PluginRegistry::get(&self, name: &str) -> Option<&Plugin>
PluginRegistry::list(&self) -> Vec<&str>

// Safety checks
PluginRegistry::should_block(&self, command: &str) -> Option<String>

// Tool aggregation
PluginRegistry::tool_registrations(&self) -> Vec<(String, String)>  // (tool_name, plugin_name)
```

## Configuration

### Plugin manifest (`{data_dir}/plugins/{name}/plugin.toml`)

```toml
name = "my-plugin"
version = "0.1.0"
author = "you"

[hooks]
pre_prompt = "scripts/pre_prompt.sh"
post_response = "scripts/post_response.sh"
on_error = "scripts/on_error.sh"
tool_block = ["rm -rf /", "format filesystem"]
tool_rewrite = []

[tools]
# Each tool is a [[tools]] array entry
[[tools]]
name = "deploy"
command = "scripts/deploy.sh"
required_level = 2
requires_approval = true

[[messaging_platforms]]
name = "teams"
enabled = true
```

### Directory structure

```
{data_dir}/plugins/
├── my-plugin/
│   ├── plugin.toml
│   └── scripts/
│       ├── pre_prompt.sh
│       ├── post_response.sh
│       └── deploy.sh
└── another-plugin/
    ├── plugin.toml
    └── ...
```

## Usage

### Creating a plugin

1. Create a directory under `{data_dir}/plugins/`.
2. Add a `plugin.toml` manifest.
3. Place any scripts referenced in the manifest alongside it.
4. Restart Praxis (or trigger a plugin reload).

### In code

```rust
use crate::plugins::PluginRegistry;

let mut registry = PluginRegistry::new(&paths);
registry.load_all()?;

// Check if a command should be blocked
if let Some(reason) = registry.should_block("rm -rf /") {
    log::warn!("command blocked: {reason}");
}

// List all registered tools
for (tool_name, plugin_name) in registry.tool_registrations() {
    println!("{plugin_name}: {tool_name}");
}
```

### Plugin loading behavior

- Invalid manifests are logged as warnings but don't prevent other plugins from loading.
- The plugins directory is created automatically if it doesn't exist.
- Plugin names must be unique; later-loaded plugins with the same name overwrite earlier ones.

## Data Files

| File | Purpose |
|---|---|
| `{data_dir}/plugins/*/plugin.toml` | Plugin manifest definitions. |
| `{data_dir}/plugins/*/scripts/*` | Shell scripts referenced by hooks and tools. |

## Dependencies

- **`paths`** — `PraxisPaths.data_dir` for resolving the plugins directory.

## Source

`src/plugins/mod.rs`
