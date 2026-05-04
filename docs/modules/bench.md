# Bench

> Capability benchmarking with longitudinal tracking

## Overview

The `bench` module provides a framework for defining, running, and tracking capability benchmarks over time. Unlike evals (which check correctness and trust on every session), benchmarks are **longitudinal tests** that measure whether the agent's capabilities improve, degrade, or stay stable across sessions.

Benchmark cases are JSON files in `data_dir/benchmarks/`. Each case defines a named, versioned scenario with one or more shell commands that must all exit successfully for the benchmark to pass. Results are appended to `benchmark_log.jsonl` as JSONL records, enabling historical trend analysis.

The distinction from evals is important:
- **Evals** — Identity and trust correctness checks, run on every session, stored per-session.
- **Benchmarks** — Capability tracking, run on demand or per schedule, stored in an append-only log.

## Architecture

### Key Types

- **`BenchmarkCase`** — A case definition: `id`, `name`, `description`, `version` (semver, defaults to "1.0.0"), `commands` (shell commands), and `tags` (for grouping).
- **`BenchmarkResult`** — A recorded result: case ID/name/version, status (`Passed`/`Failed`/`Error`), summary, timestamp, and tags.
- **`BenchmarkStatus`** — `Passed`, `Failed`, or `Error`.
- **`BenchmarkSuite`** — Stateless runner that validates cases, executes them, and appends results to the log.

### Execution Model

1. Scan `benchmarks/*.json` for case files.
2. For each case, run all commands sequentially via `/bin/sh -lc`.
3. If any command exits non-zero → `Failed`.
4. If any command fails to execute → `Error`.
5. All commands succeed → `Passed`.
6. Append results to `benchmark_log.jsonl`.

## Public API

```rust
// Running benchmarks
BenchmarkSuite.validate(paths) -> Result<usize>
BenchmarkSuite.run(paths) -> Result<Vec<BenchmarkResult>>
BenchmarkSuite::load_log(paths) -> Result<Vec<BenchmarkResult>>

// Summaries
summarize_results(results) -> String
result.status_label() -> &'static str  // "pass", "FAIL", "ERROR"
```

## Configuration

No `praxis.toml` fields. Benchmark cases are self-describing JSON files.

### Benchmark Case Format

```json
{
  "id": "memory-recall",
  "name": "Memory Recall Accuracy",
  "description": "Verify the agent can recall stored memories accurately",
  "version": "1.0.0",
  "commands": [
    "praxis memory search 'operator preferences' | grep -q 'CLI-first'"
  ],
  "tags": ["memory", "reasoning"]
}
```

Each command is run via `/bin/sh -lc <command>` with the working directory set to `data_dir`.

## Usage

### CLI

```bash
# Run all benchmarks
praxis bench run

# Validate benchmark case files
praxis bench validate

# View benchmark history
praxis bench log
```

## Data Files

| File | Purpose |
|---|---|
| `data_dir/benchmarks/*.json` | Benchmark case definitions |
| `data_dir/benchmark_log.jsonl` | Append-only result log |

## Dependencies

- `paths` — `PraxisPaths` (for `benchmarks_dir` and `benchmark_log_file`)

## Source

`src/bench/`
