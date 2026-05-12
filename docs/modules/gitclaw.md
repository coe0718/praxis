# Gitclaw

> Git-native agent lifecycle — identity, rules, memory, and capabilities as version-controlled files for distributed sharing and collaboration.

## Overview

The Gitclaw module provides a framework for managing agent state (identity, rules, memory snapshots, capabilities) inside a git repository. This enables versioning, branching, forking, and remote synchronisation of agent configuration — turning agent state into a git-native primitive.

The module defines data types for git-tracked agent identity (`GitAgentIdentity`), policy rules (`GitAgentRules`, `GitRule`), memory snapshots (`GitMemorySnapshot`), and capabilities (`GitCapabilities`). The `Gitclaw` manager struct provides lifecycle operations: `init`, `commit_state`, `push`, `pull`, `load_identity`, `load_rules`, and `fork_from`.

**Current status:** Data types and manager scaffolding are implemented. Git operations are placeholders returning mock values.

## Architecture

### Key Types

| Type | Description |
|------|-------------|
| `GitAgentIdentity` | Agent identity stored in git (agent_id, name, created_at, branch, remote_url). |
| `GitAgentRules` | Versioned ruleset with an Ed25519 signature. |
| `GitRule` | A single rule: id, pattern, action, priority, active flag. |
| `GitMemorySnapshot` | A memory state snapshot linked to a git commit hash. |
| `GitCapabilities` | Core skills, available tools, and specializations. |
| `Gitclaw` | The manager struct for git lifecycle operations. |

### Relationships

`Gitclaw` holds a repo path and main branch name. It produces and consumes the typed data structs through its lifecycle methods.

## Public API

### `GitAgentIdentity`

```rust
pub struct GitAgentIdentity {
    pub agent_id: String,
    pub name: String,
    pub created_at: String,
    pub branch: String,
    pub remote_url: Option<String>,
}
```

### `GitAgentRules`

```rust
pub struct GitAgentRules {
    pub version: String,
    pub rules: Vec<GitRule>,
    pub signature: String,
}
```

The `signature` field is intended for Ed25519 signing of the ruleset to ensure integrity.

### `GitRule`

```rust
pub struct GitRule {
    pub id: String,
    pub pattern: String,
    pub action: String,
    pub priority: u32,
    pub active: bool,
}
```

### `GitMemorySnapshot`

```rust
pub struct GitMemorySnapshot {
    pub timestamp: String,
    pub commit_hash: String,
    pub memory_state: serde_json::Value,
    pub size_bytes: usize,
}
```

### `GitCapabilities`

```rust
pub struct GitCapabilities {
    pub core_skills: Vec<String>,
    pub available_tools: Vec<String>,
    pub specializations: Vec<String>,
}
```

### `Gitclaw`

```rust
impl Gitclaw {
    pub fn init(repo_path: &Path) -> Result<Self>
    pub fn commit_state(
        &self,
        message: &str,
        identity: &GitAgentIdentity,
        rules: &GitAgentRules,
    ) -> Result<String>
    pub fn push(&self) -> Result<()>
    pub fn pull(&self) -> Result<()>
    pub fn load_identity(&self) -> Result<Option<GitAgentIdentity>>
    pub fn load_rules(&self) -> Result<Option<GitAgentRules>>
    pub fn fork_from(&self, source_url: &str, branch: &str) -> Result<GitAgentIdentity>
}
```

- **`init`** — Initialises gitclaw in a repository path. Would run `git init` and set up branches in production.
- **`commit_state`** — Commits identity and rules as version-controlled files. Returns a commit hash (placeholder: `"commit_hash_placeholder"`).
- **`push`** — Pushes the current branch to the configured remote.
- **`pull`** — Pulls from the configured remote.
- **`load_identity`** — Reads `GitAgentIdentity` from git (placeholder: returns `None`).
- **`load_rules`** — Reads `GitAgentRules` from git (placeholder: returns `None`).
- **`fork_from`** — Forks another agent's configuration by cloning their repo at `source_url` on the given branch. Returns a new `GitAgentIdentity` with a forked agent ID.

## Configuration

No TOML config. The repo path is passed directly to `Gitclaw::init()`:

```rust
let gitclaw = Gitclaw::init(&paths.data_dir)?;
```

## Usage

```rust
use praxis::gitclaw::{
    Gitclaw, GitAgentIdentity, GitAgentRules, GitRule,
};
use std::path::Path;

let gitclaw = Gitclaw::init(Path::new("/path/to/agent/repo"))?;

let identity = GitAgentIdentity {
    agent_id: "agent_001".into(),
    name: "My Agent".into(),
    created_at: "2026-05-06T00:00:00Z".into(),
    branch: "main".into(),
    remote_url: None,
};

let rules = GitAgentRules {
    version: "1.0".into(),
    rules: vec![GitRule {
        id: "rule_001".into(),
        pattern: "email:urgent".into(),
        action: "escalate".into(),
        priority: 10,
        active: true,
    }],
    signature: "ed25519_sig_here".into(),
};

let commit = gitclaw.commit_state("Initial commit", &identity, &rules)?;
gitclaw.push()?;

// Fork another agent's configuration
let forked = gitclaw.fork_from("https://github.com/user/agent-repo", "main")?;
```

## Data Files

| File | Purpose |
|------|---------|
| `agent_repo/` | Git repository containing identity, rules, memory, and capabilities as tracked files. |

## Dependencies

- **`serde` / `serde_json`** — Serialization of all git-tracked data types.
- **`chrono`** — Timestamp generation for `fork_from`.
- **`anyhow`** — Error handling.

## Source

`src/gitclaw.rs`