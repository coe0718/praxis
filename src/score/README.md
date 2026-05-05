# Score

Irreplaceability score — a per-session composite metric that quantifies how

Irreplaceability score — a per-session composite metric that quantifies how

## Features

This module provides the following components:

- `ScoreWeights`
- `SessionScoreInput`
- `SessionScore`
- `compute`
- `with_session_id`
- `summary_line`
- `record_score`
- `load_recent_scores`
- `rolling_composite`


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
cargo test --package praxis-score --lib
```


## Usage

As a library module, score is typically used internally by the Praxis framework. 
Consult the source code and module documentation for specific usage patterns.

### Importing
In your Rust code:
```rust
use praxis::score::*;  // or specific components as needed
```

### Configuration
Configuration for this module is typically handled through the main Praxis configuration files:
- `praxis.toml` - Main application configuration
- Component-specific TOML files in the `config/` directory (if applicable)


## Configuration

This module follows Praxis' standard configuration patterns:
- Primary configuration via `praxis.toml`
- Runtime configuration through context and state
- Component-specific settings may be available through environment variables or TOML files

Consult the source code and the main Praxis README for detailed configuration options.


## API Reference

### Main Components
- `ScoreWeights`
- `SessionScoreInput`
- `SessionScore`
- `compute`
- `with_session_id`
- `summary_line`
- `record_score`
- `load_recent_scores`
- `rolling_composite`

### Public Interface
This module exposes its functionality through public structs, enums, traits, and functions as defined in its source code.

### Dependencies
Consult Cargo.toml in the Praxis root directory for this module's dependencies.


## Examples

See the Praxis repository's `examples/` directory (if present) or test files for usage examples.
For specific examples of this module in action, examine:
- Integration tests in `tests/`
- Unit tests within the module (if present)
- The main Praxis CLI and runtime code


## Current Status

✅ **This module is fully implemented and functional.**

This module contains complete functionality as part of the Praxis AI agent framework.


## Related Modules

This module is part of the Praxis AI agent framework. See the main Praxis README for 
an overview of the architecture and how modules interact.

Key related modules include:
- `loop` - Session runtime and phase orchestration
- `context` - Budget engine and context assembly  
- `tools` - Tool registry, policy, and approval flow
- `memory` - Memory storage and retrieval
- `cli` - Command-line interface


---

*Documentation auto-generated for Praxis module `score`*
