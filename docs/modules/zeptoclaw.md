# ZeptoClaw

> 32 built-in tools for Praxis — an expanded tool inventory beyond the core 4 (file-read, git-query, shell-exec, web-fetch).

## Overview

The ZeptoClaw module defines a `ToolInventory` of 32 named tools organised into 6 categories: FileSystem, Network, Data, Communication, Development, and Utility. Each tool is described by a `ToolDef` with an `id`, `description`, `category`, and a `requires_auth` flag.

This inventory serves as a registry that the tool system can query — by category, by ID, or as a complete list — for tool discovery and approval routing.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `ToolDef` | A single tool definition with id, description, category, and auth requirement. |
| `ToolCategory` | Enum: `FileSystem`, `Network`, `Data`, `Communication`, `Development`, `Utility`. |
| `ToolInventory` | Registry of all 32 built-in tools with query methods. |

## Public API

### `ToolDef`

```rust
pub struct ToolDef {
    pub id: String,
    pub description: String,
    pub category: ToolCategory,
    pub requires_auth: bool,
}
```

### `ToolCategory`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCategory {
    FileSystem,
    Network,
    Data,
    Communication,
    Development,
    Utility,
}
```

### `ToolInventory`

```rust
impl ToolInventory {
    pub fn new() -> Self
    pub fn list_all(&self) -> &[ToolDef]
    pub fn by_category(&self, cat: ToolCategory) -> Vec<&ToolDef>
    pub fn find(&self, id: &str) -> Option<&ToolDef>
}
```

- **`new`** — Constructs the full inventory of 32 tools.
- **`list_all`** — Returns a slice of all tool definitions.
- **`by_category`** — Filters tools by their category.
- **`find`** — Looks up a tool by its string ID (e.g., `"file-read"`, `"docker-run"`).

### Complete Tool Inventory

| # | ID | Category | Auth Required |
|---|----|----------|---------------|
| 1 | `file-read` | FileSystem | No |
| 2 | `git-query` | Development | No |
| 3 | `shell-exec` | Utility | Yes |
| 4 | `web-fetch` | Network | No |
| 5 | `file-write` | FileSystem | Yes |
| 6 | `file-list` | FileSystem | No |
| 7 | `file-search` | FileSystem | No |
| 8 | `file-delete` | FileSystem | Yes |
| 9 | `code-analyze` | Development | No |
| 10 | `cargo-check` | Development | Yes |
| 11 | `npm-install` | Development | Yes |
| 12 | `docker-run` | Development | Yes |
| 13 | `git-commit` | Development | Yes |
| 14 | `git-push` | Development | Yes |
| 15 | `env-var` | Development | Yes |
| 16 | `http-post` | Network | No |
| 17 | `email-send` | Communication | Yes |
| 18 | `sms-send` | Communication | Yes |
| 19 | `discord-send` | Communication | Yes |
| 20 | `slack-post` | Communication | Yes |
| 21 | `json-parse` | Data | No |
| 22 | `csv-read` | Data | No |
| 23 | `sql-query` | Data | Yes |
| 24 | `embedding-create` | Data | Yes |
| 25 | `crypto-hash` | Utility | No |
| 26 | `time-now` | Utility | No |
| 27 | `cron-parse` | Utility | No |
| 28 | `url-encode` | Utility | No |
| 29 | `base64-encode` | Utility | No |
| 30 | `uuid-generate` | Utility | No |
| 31 | `random-int` | Utility | No |
| 32 | `url-shorten` | Network | No |

## Configuration

No direct configuration. The inventory is hardcoded at compile time. Individual tool approval levels and sandboxing are configured via `praxis.toml` tool manifests in the main tool system.

## Usage

```rust
use praxis::zeptoclaw::{ToolInventory, ToolCategory};

let inventory = ToolInventory::new();

// List all tools
for tool in inventory.list_all() {
    println!("{} - {}", tool.id, tool.description);
}

// Find tools by category
let dev_tools = inventory.by_category(ToolCategory::Development);

// Look up a specific tool
if let Some(tool) = inventory.find("file-read") {
    assert!(!tool.requires_auth);
}
```

## Data Files

None. The inventory is compiled into the binary.

## Dependencies

- **`serde`** — Serialization for `ToolDef` and `ToolCategory`.

## Source

`src/zeptoclaw.rs`