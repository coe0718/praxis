# Context

The Context module manages the agent's working memory, handling context assembly, budgeting, compaction, and handoff between sessions.

The Context module manages the agent's working memory, handling context assembly, budgeting, compaction, and handoff between sessions.

## Features

This module provides the following components:

- `BudgetedContext` - Context with token budget tracking
- `BudgetedSource` - Context source with budget constraints
- `ContextBudgeter` - Manages token budgets for context assembly
- `ContextSourceInput` - Input types for context sources
- `ContextCache` - Caching mechanism for context entries
- `ContextCacheEntry` - Individual cached context items
- `CompactionRequest` - Request to compact context
- `CompactionTrigger` - Triggers for automatic compaction
- `TrackedContextReader` - Reads context with file tracking
- `ContextLoadRequest` - Request to load context from various sources
- `LocalContextLoader` - Loads context from local files

## Installation & Setup

This module is part of the Praxis AI agent framework and is installed automatically when you build Praxis.

### Prerequisites
- Rust toolchain (stable)
- Cargo package manager
- Dependencies listed in the root Cargo.toml

### Building
From the Praxis root directory:
```bash
cargo build --release
```

### Testing
```bash
cargo test --package praxis-context --lib
```


## Usage

The Context module is used throughout the Praxis runtime to manage the agent's working memory and context window. It is primarily accessed by the Orient phase and during context assembly operations.

### Importing

In your Rust code:
```rust
use praxis::context::{BudgetedContext, ContextBudgeter, ContextCache};
// or specific components as needed
```

### Context Assembly

The module provides functions to assemble context from various sources while respecting token budgets:
- `adapt_config()` - Adjust configuration based on context feedback
- `record_context_feedback()` - Record how context was used for learning
- `compact_if_needed()` - Automatically compact context when thresholds are exceeded
- `load_context_cache()` / `write_context_cache()` - Persist context cache to disk

## Configuration

This module follows Praxis' standard configuration patterns:
- Primary configuration via `praxis.toml`
- Runtime configuration through context and state
- Component-specific settings may be available through environment variables or TOML files

Consult the source code and the main Praxis README for detailed configuration options.


## API Reference

### Main Components

- `BudgetedContext` - Context wrapper with budget tracking
  - `new()` - Create empty context
  - `add_source()` - Add a context source with token cost
  - `used_tokens()` - Get tokens used so far
  - `remaining_budget()` - Get tokens remaining in budget

- `ContextBudgeter` - Manages token budgets
  - `new(limit)` - Create budgeter with token limit
  - `can_add(cost)` - Check if adding cost fits in budget
  - `consume(cost)` - Consume tokens from budget

- `ContextCache` - Persistent context caching
  - `load_context_cache(path)` - Load cache from disk
  - `write_context_cache(path)` - Save cache to disk
  - `render_context_cache()` - Format cache for display

### Context Operations

- `compact_if_needed(store, trigger)` - Auto-compact based on triggers
- `request_compact(store, request)` - Manually request compaction
- `adapt_config(feedback)` - Adjust config based on usage
- `record_context_feedback(event)` - Record how context was consumed

### Dependencies

This module depends on:
- `tokenizers` - For token counting
- `storage` - For persisting context cache
- `time` - For tracking context age

## Examples

### Basic Context Budgeting

```rust
use praxis::context::{ContextBudgeter, BudgetedContext};
use praxis::tokenizers::TokenCounter;

// Create a budgeter with 4096 token limit
let budgeter = ContextBudgeter::new(4096);

// Add context sources with their token costs
let sources = vec![
    ("system_prompt".to_string(), 150),
    ("recent_memories".to_string(), 800),
    ("current_task".to_string(), 200),
];

let mut context = BudgetedContext::new();
for (source, cost) in sources {
    if budgeter.can_add(cost) {
        context.add_source(source, cost)?;
        budgeter.consume(cost);
    }
}

println!("Context has {} tokens used", context.used_tokens());
```

### Context Compaction

```rust
use praxis::context::{compact_if_needed, CompactionTrigger};
use praxis::storage::SessionStore;

// Check if context needs compaction
if compact_if_needed(&store, CompactionTrigger::TokenLimit)? {
    println!("Context was compacted to stay within budget");
}

// Manual compaction request
let request = CompactionRequest {
    trigger: CompactionTrigger::Manual,
    target_ratio: 0.7,  // Aim for 70% of max tokens
};
request_compact(&store, request)?;
```

## Current Status


✅ **This module is fully implemented and functional.**

This module contains complete functionality as part of the Praxis AI agent framework.

## Related Modules

This module is part of the Praxis AI agent framework. See the main Praxis README for 
an overview of the architecture and how modules interact.

Key related modules include:
- `loop` - Session runtime and phase orchestration
- `tools` - Tool registry, policy, and approval flow
- `memory` - Memory storage and retrieval
- `cli` - Command-line interface


---

