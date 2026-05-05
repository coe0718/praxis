# Loop

The Loop module orchestrates the four-phase agent cycle (Orient → Decide → Act → Reflect → Sleep) that drives Praxis autonomous behavior.

The Loop module orchestrates the four-phase agent cycle (Orient → Decide → Act → Reflect → Sleep) that drives Praxis autonomous behavior.

## Features

This module provides the following components:

- `RunOptions`
- `RunSummary`


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
cargo test --package praxis-loop --lib
```


## Usage

The Loop module is the core of Praxis runtime, managing session execution through its four-phase cycle. It is instantiated by the main binary and not typically used directly by external code. 
Consult the source code and module documentation for specific usage patterns.

### Importing
In your Rust code:
```rust
use praxis::loop::*;  // or specific components as needed
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
- `RunOptions`
- `RunSummary`

### Public Interface
This module exposes its functionality through public structs, enums, traits, and functions as defined in its source code.

### Dependencies
Consult Cargo.toml in the Praxis root directory for this module's dependencies.


## Examples

```rust
use praxis::loop::{PraxisRuntime, RunOptions};
use praxis::config::AppConfig;
use praxis::paths::PraxisPaths;

// In your main or test setup
let config = AppConfig::load()?;
let paths = PraxisPaths::new(&config.data_dir)?;

// Create runtime (you'll need to provide the generic types)
let runtime = PraxisRuntime {
    config: &config,
    paths: &paths,
    // ... provide backend, clock, events, etc.
};

// Run one session cycle
let options = RunOptions {
    once: true,
    force: false,
    task: Some("check email".to_string()),
};

let summary = runtime.run_once(options)?;
println!("Session completed: {}", summary.outcome);
```

## Current Status

✅ **This module is fully implemented and functional.**

This module contains complete functionality as part of the Praxis AI agent framework.


## Related Modules

This module is part of the Praxis AI agent framework. See the main Praxis README for 
an overview of the architecture and how modules interact.

Key related modules include:
- `context` - Budget engine and context assembly  
- `tools` - Tool registry, policy, and approval flow
- `memory` - Memory storage and retrieval
- `cli` - Command-line interface


---


