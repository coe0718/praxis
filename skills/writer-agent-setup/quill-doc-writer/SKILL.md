---
name: quill-doc-writer
description: "Quill's documentation writing workflow — READMEs, module docs, architecture docs, setup guides for Rust/Python projects. Professional, end-user-facing output."
autoload: true
---

# Quill Documentation Writer

Write documentation that people actually read and understand. This skill defines the templates, style, and process for all documentation work.

Quill runs on MiniMax M2.5 (free via OpenRouter) — Nemotron 3 Super is too weak for quality doc writing, it produces boilerplate template output with duplicated lines and generic filler. Loaded via `skills.config.quill-doc-writer: {}` in config.yaml.

## Documentation Templates

### 1. README File

```
# Project Name

Brief description (1-2 sentences about what this project does and who it's for).

## Features

- Feature 1: what it does
- Feature 2: what it does
- Feature 3: what it does

## Quick Start

\`\`\`bash
# Prerequisites and install steps
\`\`\`

## Usage

Brief usage examples with expected output.

## Configuration

Key configuration options with descriptions and default values.

## Architecture

High-level architecture overview (2-3 paragraphs).

## Contributing

How to contribute (or pointer to CONTRIBUTING.md).

## License
```

### 2. Module Documentation

```
# Module Name

Purpose and responsibility of this module.

## Overview

What this module does, why it exists, what problem it solves.

## Key Types and Traits

### `TypeName`

Description, fields, and usage.

### `TraitName`

Description, methods, and implementors.

## Configuration

Config fields this module reads, with types and defaults.

## Usage

\`\`\`rust
// Working code example
\`\`\`

## Data Files

Files this module reads or writes, with paths and formats.

## Dependencies

Modules this module depends on and why.
```

### 3. Architecture Document

```
# Architecture Overview

High-level description of the system.

## Domain Map

Group modules by responsibility domain. List each module with one-line summary.

## Data Flow

Description of how data moves through the system.

## Key Design Decisions

Notable architectural choices and why they were made.

## Dependency Graph

Description of module interdependencies.
```

## Quality Gates (Hard Rules)

These are NOT optional. Breaking any of these is a failing grade.

### 1. NO Duplicate First Lines

The first paragraph of every README appears exactly once. "X module for Y framework" repeated on lines 3 and 5 is a bug, not a feature. If you wrote duplicate content, you are templating, not writing. Fix it.

### 2. NO "See source code" in Feature Lists

Every listed component gets a one-sentence description of what it does. If you write "See source code for detailed component listing," you have failed. The README IS the documentation — "go read the code" means the README is useless. Describe each component's purpose. If you can't, don't list it.

### 3. Implemented ≠ Stubs

Three tiers of documentation thickness:
- **IMPLEMENTED (500+ meaningful LOC):** Full architecture, usage examples, configuration reference, all component descriptions. This is a serious module — it gets serious docs.
- **PARTIAL (<500 LOC, working):** Honest about what works, what doesn't. Include examples for what's functional, mark missing pieces clearly.
- **STUB:** Brief (3-4 sentences). Honest about state: "This module is a placeholder. It registers itself but has no functionality yet. Status: stub."

If you reclassify a module from STUB to IMPLEMENTED (which is good — you read the code), its README must be NOTICEABLY richer than the stubs. A reader should be able to tell which modules are real and which aren't just by reading the README.

### 4. At Least One Real Code Example per Implemented Module

Reading the source means extracting actual usage. At minimum:
```rust
// Copy-paste the actual import path and one real function call from the module
use praxis::context::ContextBuilder;
let ctx = ContextBuilder::new().build();
```

Not a fake example. Not pseudocode. Not "see tests for examples." Real code that compiles, using the actual function signatures you read.

### 5. No Boilerplate Installation Sections

"Rust toolchain (stable)" and "Cargo package manager" do not need to appear in 64 READMEs. If every module has the same installation boilerplate, link to the root build instructions once and be done. Each README should only mention module-specific prerequisites.

### 6. Filesystem Cleanup After Conversion

If you restructure files (e.g., converting `.rs` → `mod.rs` directories), you MUST delete the old files. Leaving both `src/foo.rs` AND `src/foo/mod.rs` causes `error[E0761]: file for module found at both locations` — 23 compiler errors. Clean up after yourself.

Verify with `ls src/{module_name}.rs src/{module_name}/mod.rs 2>&1` — if both exist, delete the old flat file.

## Style Guide

- **Write for the end user** — someone who just cloned the repo and has zero context
- **Verify from source** — always check actual type signatures, config keys, and file paths before writing
- **One sentence per concept** — break complex ideas into digestible pieces
- **Code blocks work** — test that examples compile. Note limitations honestly
- **Tables for config** — when a module has 3+ config options, use a table
- **No filler** — "This module is responsible for..." → say what it *does*
- **Markdown only** — all docs in `.md` files
- **Tone** — professional but not academic. Confident without hype. Think Stripe docs.
- **Model: Use MiniMax M2.5 or better.** Nemotron 3 Super produces template boilerplate with duplicated lines. If you're on a weaker model, expect to write twice as many revisions.

## Verification Checklist (Post-Quality-Gates)

Before marking a doc complete, after the quality gates above are satisfied:
- [ ] First paragraph is NOT duplicated (check line 1 vs line 3)
- [ ] Every component in the Features section has a real description, not "see source code"
- [ ] README thickness matches module state (implemented gets rich docs, stubs get brief)
- [ ] At least one real code example extracted from actual source
- [ ] No unnecessary boilerplate installation section
- [ ] No stale `.rs` files left behind after restructuring
- [ ] Tone is professional, not promotional
- [ ] No markdown rendering issues
