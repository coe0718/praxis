# Merkle

> Hash-chain audit trail that cryptographically links all actions so tampering with any past record invalidates the chain.

## Overview

The Merkle module provides a tamper-evident audit log for all Praxis agent actions. Every significant operation — tool executions, config changes, identity mutations — is appended as an entry in an append-only JSONL file. Each entry contains a SHA-256 hash computed over the entry's contents plus the previous entry's hash, forming a cryptographic chain.

This design means that modifying or deleting any historical record invalidates every subsequent hash, making unauthorized changes detectable. The latest root hash can be compared across backups or independent replicas to verify data integrity. The verification is purely local — no network or distributed consensus is needed.

The module is intentionally minimal: it provides append, load, verify, and a latest-hash query. There is no deletion API, reinforcing the append-only guarantee at the code level.

## Architecture

### `AuditEntry` (struct)

A single entry in the audit chain.

| Field | Type | Description |
|---|---|---|
| `seq` | `u64` | Monotonically increasing sequence number (1-based). |
| `timestamp` | `String` | RFC 3339 UTC timestamp when the entry was created. |
| `action` | `String` | Human-readable label for the action (e.g. `"tool.execute"`). |
| `payload_json` | `String` | JSON-serialized payload describing the action. |
| `prev_hash` | `String` | Hex-encoded SHA-256 hash of the previous entry (empty string for the first). |
| `hash` | `String` | Hex-encoded SHA-256 hash of this entry. |

### `MerkleTrail` (struct)

Append-only audit log backed by a JSONL file. Holds only the file path; entries are read from disk on each operation.

### Hash computation

`compute_hash(seq, timestamp, action, payload, prev_hash)` concatenates all fields and produces a SHA-256 digest encoded as lowercase hex.

## Public API

```rust
// Constructor
MerkleTrail::new(path: &Path) -> Self

// Append a new entry and return its hash
MerkleTrail::append(&self, action: &str, payload: &serde_json::Value) -> Result<String>

// Load all entries from disk
MerkleTrail::load(&self) -> Result<Vec<AuditEntry>>

// Verify chain integrity (returns false if any hash is invalid)
MerkleTrail::verify(&self) -> Result<bool>

// Get the latest hash in the chain
MerkleTrail::latest_hash(&self) -> Result<Option<String>>
```

## Configuration

The Merkle module is configured through `PraxisPaths` — the audit trail file path is set during Praxis initialization. No `praxis.toml` fields, environment variables, or CLI flags are specific to this module.

## Usage

```rust
use crate::merkle::MerkleTrail;

let trail = MerkleTrail::new(&paths.audit_trail_file);

// Record an action
let hash = trail.append("tool.execute", &serde_json::json!({
    "tool": "shell-exec",
    "command": "ls -la"
}))?;

// Verify chain integrity
if !trail.verify()? {
    log::error!("audit trail integrity check failed!");
}

// Get the latest hash for cross-backup comparison
let root = trail.latest_hash()?;
```

## Data Files

| File | Format | Purpose |
|---|---|---|
| `{data_dir}/audit_trail.jsonl` | JSONL (one `AuditEntry` per line) | Append-only audit log. |

## Dependencies

- **`sha2`** — SHA-256 hashing.
- **`chrono`** — Timestamps.
- **`serde_json`** — Payload serialization and entry deserialization.
- Used by: the Praxis runtime loop for recording significant actions during the Act and Reflect phases.

## Source

`src/merkle.rs`
