# Wave

> Dependency-aware parallel execution via topological wave planning.

## Overview

The wave module implements a structured approach to parallelism: instead of spawning work ad hoc, you define a dependency graph of work items and the engine groups them into sequential **waves**. Within each wave, every node is independent — no intra-wave dependencies exist — so all nodes in a wave can theoretically execute in parallel. Across waves, ordering is guaranteed by the dependency graph.

This pattern is particularly useful for large "gap closure" tasks where many independent sub-tasks must be completed, some of which depend on others. The wave planner automatically determines the maximum parallelism possible while respecting all ordering constraints.

If a node fails, all nodes that depend on it (directly or transitively) in subsequent waves are marked `Skipped`. Independent nodes in the same wave continue to execute normally.

## Architecture

### Core types

| Type | Description |
|------|-------------|
| `WaveNode` | A unit of work: unique `id`, human-readable `description`, and a list of `deps` (IDs of prerequisite nodes). |
| `WaveGraph` | A collection of `WaveNode` values forming a dependency graph. Provides `into_waves()` for topological sorting. |
| `WaveNodeResult` | Outcome of executing one node: `wave_index`, `node_id`, `outcome`, `detail` string. |
| `WaveOutcome` | Enum: `Success`, `Failed`, `Skipped`. |

### Algorithm

The wave planner uses **Kahn's algorithm** for topological sorting, extended to produce level-grouped output:

1. **Validation** — every dependency ID must reference a known node. Unknown IDs cause an error.
2. **In-degree computation** — each node's in-degree equals the number of its dependencies.
3. **Wave formation** — all nodes with in-degree 0 form the current wave. After processing, their dependents' in-degrees are decremented. Newly-zero nodes form the next wave.
4. **Cycle detection** — if the total processed count doesn't equal the node count, the graph contains a cycle.

Nodes within each wave are sorted alphabetically by ID for deterministic output.

### Failure propagation

When `execute_waves()` runs:
- A failed node is recorded in the `failed_ids` set.
- Before executing any node, its deps are checked against `failed_ids`. If any dependency failed, the node is marked `Skipped` and added to `failed_ids` (so its own dependents are also skipped).
- Nodes in the same wave that are independent of the failed node still execute normally.

### Example

```
A ──► C ──► E
            ▲
B ──► D ────┘
```

- Wave 0: `[A, B]` (no dependencies)
- Wave 1: `[C, D]` (A→C, B→D)
- Wave 2: `[E]` (depends on C and D)

If C fails, E is skipped, but D still runs (it's in the same wave and doesn't depend on C).

## Public API

### Building a graph

```rust
use crate::wave::{WaveNode, WaveGraph};

let graph = WaveGraph::new(vec![
    WaveNode::new("a", "Initialize database"),
    WaveNode::new("b", "Fetch remote config"),
    WaveNode::new("c", "Run migrations").with_deps(["a"]),
    WaveNode::new("d", "Apply config").with_deps(["b"]),
    WaveNode::new("e", "Start server").with_deps(["c", "d"]),
]);
```

### Topological sort into waves

```rust
let waves: Vec<Vec<WaveNode>> = graph.into_waves()?;
for (i, wave) in waves.iter().enumerate() {
    println!("Wave {i}: {:?}", wave.iter().map(|n| &n.id).collect::<Vec<_>>());
}
```

### Execution with failure propagation

```rust
use crate::wave::execute_waves;

let results = execute_waves(graph, |node| {
    // Your execution logic here
    // Return Ok(detail_string) on success, Err on failure
    do_work(node)
})?;

for r in &results {
    println!("{}: {:?} — {}", r.node_id, r.outcome, r.detail);
}
```

### Summary

```rust
use crate::wave::summarize_waves;

let summary = summarize_waves(&results);
// "Wave execution: 4/5 succeeded, 1 failed, 0 skipped"
```

## Configuration

No configuration files, environment variables, or feature flags. The wave module is a pure computation library.

## Usage

The wave module is used programmatically within Praxis for batch operations. There is no dedicated CLI command.

Typical integration points:
- **Act phase** — decomposing a complex goal into parallel sub-tasks.
- **Maintenance** — running multiple independent cleanup operations concurrently.
- **Batch tool execution** — executing a dependency-ordered set of tool calls.

## Data Files

No data files. The wave module operates entirely in memory.

## Dependencies

- **`serde`** — serialization of `WaveNode`, `WaveGraph`, `WaveNodeResult`, and `WaveOutcome`.
- **`anyhow`** — error handling for cycle detection and unknown dependency errors.
- No dependency on other Praxis modules — the wave module is self-contained.

## Source

`src/wave/`
