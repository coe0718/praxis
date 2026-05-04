# Hands

> Installable role bundles that activate a coherent set of tools, skills, and scheduling overrides in a single step.

## Overview

Hands are Praxis's answer to role-based configuration: instead of manually enabling tools, loading skills, and tweaking quiet-hour schedules every time you want the agent to behave differently, you declare a *hand* as a single TOML file. When activated, a hand becomes the agent's current operational role, carrying its own tool requirements, skill set, and quiet-hour overrides.

Multiple hands can be installed simultaneously, but only one is considered *active* at a time. This makes it easy to switch between personas — e.g. a "reviewer" hand for code review and a "writer" hand for documentation — without editing configuration files.

The hand system is intentionally lightweight. Hands are just TOML manifests discovered from a `hands/` directory, parsed at load time, and passed to the runtime for context assembly. No database, no migration, no API surface — just files on disk.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `HandManifest` | Fully parsed hand manifest (name, description, version, tools, skills, schedule, metadata). |
| `HandTools` | Declares `required` and `optional` tool name lists. |
| `HandSkills` | Lists skill file base-names (without `.md`) to load when this hand is active. |
| `HandSchedule` | Override quiet-hours (0–23) for this hand; empty inherits the global schedule. |
| `HandMetadata` | Free-form `author` string and `tags` list. |
| `HandStore` | Collection of all installed hands, loaded by scanning the `hands/` directory. |

### Relationships

`HandStore` owns a `Vec<HandManifest>`. Each `HandManifest` contains `HandTools`, `HandSkills`, `HandSchedule`, and `HandMetadata`. The `HandStore` is constructed by scanning a directory for `*.toml` files and parsing each one into a `HandManifest`.

## Public API

### `HandManifest`

```rust
pub fn load(path: &Path) -> Result<Self>
pub fn summary(&self) -> String
```

- **`load`** — Parse a single TOML file into a manifest. Sets `source_path` automatically.
- **`summary`** — Returns a one-line summary like `"reviewer v1.0.0 — Code review assistant (tools: 3, skills: 2)"`.

### `HandStore`

```rust
pub fn load(hands_dir: &Path) -> Result<Self>
pub fn get(&self, name: &str) -> Option<&HandManifest>
pub fn summary(&self) -> String
```

- **`load`** — Scan a directory for `*.toml` files and parse them. Invalid manifests are skipped with a warning. Results are sorted by name.
- **`get`** — Case-insensitive lookup by name.
- **`summary`** — Multi-line listing of all installed hands.

### Free Functions

```rust
pub fn install_hand(paths: &PraxisPaths, manifest: &HandManifest) -> Result<PathBuf>
pub fn remove_hand(paths: &PraxisPaths, name: &str) -> Result<bool>
```

- **`install_hand`** — Serialize a manifest and write it to `hands/<name>.toml`. Creates the directory if needed.
- **`remove_hand`** — Delete a hand file by name. Returns `true` if the file existed.

## Configuration

Hands are configured entirely through TOML files — no `praxis.toml` entries or environment variables are required.

### Manifest Format (`hands/<name>.toml`)

```toml
name        = "reviewer"
description = "Code-review assistant: reads PRs, leaves structured comments."
version     = "1.0.0"

[tools]
required = ["read_file", "web_search"]
optional = ["write_file"]

[skills]
load = ["review_pr", "summarise"]

[schedule]
quiet_hours = [0, 1, 2, 3, 4, 5]

[metadata]
author = "operator"
tags   = ["code", "review"]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | *required* | Machine-readable identifier (should match filename without `.toml`). |
| `description` | string | *required* | Human-readable summary. |
| `version` | string | `"1.0.0"` | Semantic version of the hand. |
| `tools.required` | `[string]` | `[]` | Tools that must be available for this hand. |
| `tools.optional` | `[string]` | `[]` | Tools used when available but not required. |
| `skills.load` | `[string]` | `[]` | Skill file base-names to activate. |
| `schedule.quiet_hours` | `[u8]` | `[]` | Hours (0–23) to treat as quiet; empty inherits global. |
| `metadata.author` | string | `""` | Author attribution. |
| `metadata.tags` | `[string]` | `[]` | Free-form tags for filtering. |

## Usage

### CLI

```bash
# Install a hand from a manifest file
praxis hands install reviewer.toml

# List installed hands
praxis hands list

# Remove a hand
praxis hands remove reviewer
```

### Programmatic

```rust
use praxis::hands::{HandManifest, HandStore};

// Load all hands from the data directory
let store = HandStore::load(&paths.hands_dir)?;

// Find a specific hand
if let Some(hand) = store.get("reviewer") {
    println!("{}", hand.summary());
}
```

## Data Files

| File | Purpose |
|------|---------|
| `data_dir/hands/<name>.toml` | One manifest per hand. Scanned by `HandStore::load`. |

## Dependencies

- **`paths`** — Uses `PraxisPaths::hands_dir` to locate the hands directory.

## Source

`src/hands.rs`
