# Delegation

> Agent-to-agent delegation links with concurrency caps and allow/deny task filtering.

## Overview

The delegation module enables a Praxis instance to delegate work to — or receive work from — other Praxis instances or external agents. Each delegation relationship is modelled as a `DelegationLink` with a direction (outbound, inbound, or bidirectional), a concurrency cap, and allow/deny task filters.

Links are persisted in a JSON file (`delegation_links.json`) and consulted during the Act phase. Tasks can be sent over a link to a remote agent's `delegation_queue.jsonl`, and inbound tasks can be drained from the local queue for processing.

The security model is straightforward: each link carries optional glob-pattern `allow_tasks` and `deny_tasks` lists. An empty allow list means "all tasks permitted" (subject to the deny list). Inbound links that don't pass the allow/deny check are rejected before reaching the approval queue.

**Current status:** The store, link management, queue I/O, and security filtering are fully implemented. The Act phase does not yet automatically delegate work over links; this is a store-and-infrastructure layer awaiting Act-phase integration.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `DelegationLink` | A single configured link with name, endpoint, direction, concurrency, and allow/deny lists. |
| `LinkDirection` | Enum: `Outbound`, `Inbound`, or `Bidirectional`. |
| `DelegationStore` | Full registry of links keyed by name, with active concurrency counters. |
| `DelegatedTask` | A task received from a remote agent (source, task text, link name, timestamp). |

### Relationships

`DelegationStore` owns a `HashMap<String, DelegationLink>` plus `active_counts` for tracking in-flight delegations. `DelegatedTask` is the wire format for inter-agent communication.

## Public API

### `DelegationLink`

```rust
pub fn new(name: impl Into<String>, endpoint: impl Into<String>, direction: LinkDirection) -> Self
pub fn permits(&self, task_name: &str) -> bool
```

- **`new`** — Creates a link with default concurrency (1) and empty allow/deny lists.
- **`permits`** — Checks whether a task name is allowed on this link. Deny list takes precedence over allow list. Returns `false` if the link is disabled.

### `DelegationStore`

```rust
pub fn load(path: &Path) -> Result<Self>
pub fn save(&self, path: &Path) -> Result<()>
pub fn add_link(&mut self, link: DelegationLink)
pub fn remove_link(&mut self, name: &str) -> Option<DelegationLink>
pub fn available_outbound(&self, task_name: &str) -> Vec<&DelegationLink>
pub fn acquire(&mut self, link_name: &str)
pub fn release(&mut self, link_name: &str)
pub fn summary(&self) -> String
```

- **`load` / `save`** — Persist to `delegation_links.json`. Returns empty store if file is absent.
- **`add_link` / `remove_link`** — Insert or remove a link by name.
- **`available_outbound`** — Find all outbound links that permit `task_name` and have remaining concurrency capacity.
- **`acquire` / `release`** — Increment/decrement the active counter for a link (call when delegation starts/completes).
- **`summary`** — Human-readable listing of all links with status and active counts.

### Free Functions

```rust
pub fn send_over_link(link: &mut DelegationLink, task: &str, from: &str, now: DateTime<Utc>) -> Result<()>
pub fn drain_inbound_delegation(queue_path: &Path) -> Result<Vec<DelegatedTask>>
```

- **`send_over_link`** — Write a `DelegatedTask` to a remote agent's `delegation_queue.jsonl`. Only file-based endpoints (absolute or `~/` paths) are currently supported; HTTP endpoints are not yet implemented.
- **`drain_inbound_delegation`** — Read and clear the local `delegation_queue.jsonl`. The queue is truncated after reading so entries are processed exactly once.

### `LinkDirection`

| Variant | `can_send` | `can_receive` |
|---------|-----------|--------------|
| `Outbound` | ✓ | ✗ |
| `Inbound` | ✗ | ✓ |
| `Bidirectional` | ✓ | ✓ |

## Configuration

No `praxis.toml` fields. Links are managed programmatically or via the CLI. The data file is auto-created when first saved.

### Data File Format (`delegation_links.json`)

```json
{
  "links": {
    "reviewer": {
      "name": "reviewer",
      "endpoint": "/path/to/remote/praxis/data",
      "direction": "outbound",
      "max_concurrency": 2,
      "allow_tasks": ["review*", "lint*"],
      "deny_tasks": ["*production*"],
      "enabled": true,
      "created_at": "2025-01-01T00:00:00Z",
      "last_used_at": null
    }
  },
  "active_counts": {}
}
```

## Usage

### CLI

```bash
# List delegation links
praxis delegation list

# Add a new outbound link
praxis delegation add --name reviewer --endpoint /path/to/remote --direction outbound

# Remove a link
praxis delegation remove reviewer
```

### Programmatic

```rust
use praxis::delegation::{DelegationLink, DelegationStore, LinkDirection};

let mut store = DelegationStore::load(&paths.delegation_links_file)?;

let link = DelegationLink::new("reviewer", "/remote/data", LinkDirection::Outbound);
store.add_link(link);

let outbound = store.available_outbound("review-code");
store.save(&paths.delegation_links_file)?;
```

## Data Files

| File | Purpose |
|------|---------|
| `data_dir/delegation_links.json` | Persistent registry of links and active counters. |
| `<endpoint>/delegation_queue.jsonl` | Inbound task queue at the remote agent's data directory. Written by `send_over_link`, drained by `drain_inbound_delegation`. |

## Dependencies

- **`paths`** — Uses `PraxisPaths::delegation_links_file` for the store location.

## Source

`src/delegation.rs`
