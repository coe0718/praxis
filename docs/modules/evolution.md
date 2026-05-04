# Evolution

> Meta-evolution workflow — Praxis proposes, reviews, and applies changes to its own configuration, profiles, and identity through a gated approval process.

## Overview

The `evolution` module enables Praxis to propose modifications to itself — its config, profiles, identity files, or even source code — in a heavily safety-gated, operator-controlled workflow. Proposals are generated automatically during the Reflect phase when the agent detects recurring patterns (e.g. quality drift, repeated failures). The operator then reviews, approves, and applies proposals via the CLI.

No proposal is ever applied without two explicit operator commands (`approve` then `apply`). Code proposals are never applied automatically — they emit human-readable instructions. The store uses an append-only JSONL log so the full proposal history is always auditable, and a `SELF_EVOLUTION.md` document is regenerated from the store for easy browsing.

## Architecture

### Types

| Type | Description |
|------|-------------|
| `ChangeKind` | The category of change: `Config`, `Profile`, `Identity`, or `Code`. |
| `ProposalStatus` | Lifecycle state: `Proposed` → `Approved` → `Applied` (or `Rejected`). |
| `EvolutionProposal` | A full proposal: ID, timestamps, title, motivation, change kind, change details, evidence session IDs, status, and rejection note. |
| `EvolutionStore` | Append-only JSONL store that manages proposal persistence and lifecycle transitions. |

### Change Kinds

| Kind | Auto-applicable? | Details format |
|------|-------------------|----------------|
| `Config` | Yes | TOML snippet to merge into `praxis.toml`. |
| `Profile` | Yes | Full profile TOML block. |
| `Identity` | Manual | Markdown patch or new section for `IDENTITY.md`/`GOALS.md`/`SOUL.md`. |
| `Code` | Manual only | Human-readable description/diff — never auto-applied. |

### Proposal Lifecycle

```
Proposed ──► Approved ──► Applied
   │              │
   └──► Rejected ◄┘
```

Only `Proposed` proposals can be approved. Only `Approved` proposals can be applied. Rejected proposals cannot be applied. Applied proposals cannot be rejected.

### SELF_EVOLUTION.md Renderer

`render_self_evolution_doc()` regenerates the human-readable proposal document from the store. It splits proposals into "Pending" (Proposed + Approved) and "History" (Applied + Rejected) sections.

## Public API

```rust
pub struct EvolutionStore { /* path: PathBuf */ }
impl EvolutionStore {
    pub fn new(path: PathBuf) -> Self;
    pub fn from_paths(paths: &PraxisPaths) -> Self;
    pub fn propose(&self, proposal: &EvolutionProposal) -> Result<()>;
    pub fn all(&self) -> Result<Vec<EvolutionProposal>>;
    pub fn with_status(&self, status: ProposalStatus) -> Result<Vec<EvolutionProposal>>;
    pub fn get(&self, id: &str) -> Result<Option<EvolutionProposal>>;
    pub fn approve(&self, id: &str) -> Result<EvolutionProposal>;
    pub fn reject(&self, id: &str, note: impl Into<String>) -> Result<EvolutionProposal>;
    pub fn mark_applied(&self, id: &str) -> Result<EvolutionProposal>;
}

pub fn render_self_evolution_doc(paths: &PraxisPaths) -> Result<()>;
```

## Configuration

No `praxis.toml` section is needed. Proposals are capped at 3 pending at a time (enforced by the Reflect phase, not the store). Deduplication is by title prefix.

## Usage

```bash
# List all proposals
praxis evolve list

# Show a specific proposal
praxis evolve show evo-20260414-080000

# Approve a proposal (first step)
praxis evolve approve evo-20260414-080000

# Apply an approved proposal (second step — activates the change)
praxis evolve apply evo-20260414-080000

# Reject a proposal with a reason
praxis evolve reject evo-20260414-080000 --note "Not ready yet"

# View the human-readable proposal document
cat SELF_EVOLUTION.md
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `evolution.jsonl` | JSONL | Append-only proposal log. Full audit trail. |
| `SELF_EVOLUTION.md` | Markdown | Rendered view of pending and historical proposals. Regenerated after each change. |

## Dependencies

- `paths` — `PraxisPaths` for file locations

## Source

`src/evolution.rs`
