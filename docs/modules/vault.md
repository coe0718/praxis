# Vault

> Encrypted secret storage that resolves named secrets at request time, keeping raw values out of the main configuration.

## Overview

The Vault module provides a centralized, auditable mechanism for managing secrets used throughout Praxis — API keys, OAuth tokens, database credentials, and any other sensitive values. Instead of scattering environment variable lookups across the codebase, every secret access flows through a single `vault.toml` file.

Secrets are stored as named aliases that either hold a literal value (intended for development only) or reference an environment variable with an optional fallback. This indirection means rotating a key only requires updating the environment variable or vault entry, not hunting through provider configs.

The vault file supports transparent encryption: if a `master.key` file exists alongside `vault.toml`, Praxis automatically encrypts on save and decrypts on load using the `crypto` module. In production, the vault should only contain environment-variable references, and Praxis warns at startup when literal entries are detected.

## Architecture

### `VaultEntry` (enum)

A single named secret entry. Two variants:

| Variant | Fields | Description |
|---|---|---|
| `Literal` | `value: String` | Hardcoded value — convenient for dev, flagged in production audits. |
| `EnvVar` | `env: String`, `fallback: Option<String>` | Resolved from an environment variable at runtime. `fallback` is used when the variable is unset or empty. |

### `Vault` (struct)

Top-level container holding a `HashMap<String, VaultEntry>` of named secrets. Provides load/save (with transparent encryption), resolve, and audit operations.

### Free functions

- **`resolve_with_fallback(vault, name)`** — Resolves from the vault first, then falls back to an uppercase environment variable lookup. Used for incremental migration of legacy code.
- **`audit_literals(vault)`** — Returns warning strings for every literal entry. Called at startup.

## Public API

```rust
// Load/save
Vault::load(path: &Path) -> Result<Vault>
Vault::save(&self, path: &Path) -> Result<()>

// Resolution
Vault::resolve(&self, name: &str) -> Result<String>
Vault::resolve_optional(&self, name: &str) -> Option<String>

// Mutation
Vault::set(&mut self, name: impl Into<String>, entry: VaultEntry)
Vault::remove(&mut self, name: &str) -> bool

// Auditing
Vault::literal_entries(&self) -> Vec<&str>
Vault::summary(&self) -> String  // Never includes values

// Helpers
resolve_with_fallback(vault: &Vault, name: &str) -> Result<String>
audit_literals(vault: &Vault) -> Vec<String>
```

## Configuration

### `vault.toml` (located at `{data_dir}/vault.toml`)

```toml
# Literal value — dev/test only
[secrets.my_literal]
value = "sk-abc123"

# Environment variable reference (preferred)
[secrets.anthropic_key]
env = "ANTHROPIC_API_KEY"

# Environment variable with fallback
[secrets.openai_key]
env      = "OPENAI_API_KEY"
fallback = "sk-tes...lder"
```

- If a `master.key` file exists next to `vault.toml`, the file is transparently encrypted using AES-256-GCM via the `crypto` module.
- `vault.toml` should be listed in `.gitignore`.
- Literal entries trigger startup warnings outside of development mode.

### Environment variables

No specific environment variables are required by the vault module itself. The `env` fields in vault entries reference arbitrary environment variables that hold the actual secret values.

## Usage

### CLI commands (`praxis vault`)

```bash
# List all secret names and types (values are never shown)
praxis vault list

# Add a secret backed by an environment variable
praxis vault set-env anthropic_key --env ANTHROPIC_API_KEY

# Add a secret with an env var and fallback
praxis vault set-env openai_key --env OPENAI_API_KEY --fallback "sk-placeholder"

# Add a literal secret (dev only)
praxis vault set-literal test_key --value "dev-secret"

# Resolve and print a secret's current value
praxis vault resolve anthropic_key

# Remove a secret
praxis vault remove test_key

# Audit for literal entries
praxis vault audit
```

### In code

```rust
let vault = Vault::load(&paths.vault_file)?;
let api_key = vault.resolve("anthropic_key")?;

// Migration-friendly: vault first, then env fallback
let key = resolve_with_fallback(&vault, "openai_key")?;
```

## Data Files

| File | Purpose |
|---|---|
| `{data_dir}/vault.toml` | Secret alias definitions. Encrypted if `master.key` is present. |
| `{data_dir}/master.key` | Optional encryption key for the vault file. |

## Dependencies

- **`crypto`** — Transparent encryption/decryption of the vault file.
- **`paths`** — `PraxisPaths.vault_file` points to the vault location.
- Used by: `backend/*`, `oauth/*`, `tools/execute.rs`, and anywhere secrets are needed.

## Source

`src/vault.rs`
