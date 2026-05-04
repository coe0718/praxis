# Attachments

> File attachment policies for `praxis ask --file`, controlling how oversized files are handled via reject, chunk, or summarize strategies.

## Overview

When using Praxis interactively (e.g. `praxis ask --file notes.md`), the Attachments module governs how file contents are rendered into the prompt context. Small files are included verbatim, but large files require a policy decision to avoid overwhelming the LLM context window.

Three policies are available:

- **`reject`** — Refuse files that exceed the size limit, requiring the operator to choose a different strategy or provide a smaller file.
- **`chunk`** — Split the file into multiple parts, each within the size limit, preserving all content across numbered chunks.
- **`summarize`** — Extract structural anchors (headings, list items, date lines, boundary markers) plus a truncated preview, keeping the most useful overview within bounds.

The module is intentionally text-only: binary files are rejected during UTF-8 validation.

## Architecture

### `AttachmentPolicy` (enum)

| Variant | Behavior |
|---|---|
| `Reject` | Error if the file exceeds `MAX_ATTACHMENT_BYTES` (100 KB). |
| `Chunk` | Split into 48 KB chunks preserving all content. |
| `Summarize` | Extract up to 12 anchor lines + 4,000 character preview. |

### Constants

| Constant | Value | Purpose |
|---|---|---|
| `MAX_ATTACHMENT_BYTES` | 100,000 (~100 KB) | Threshold above which policy is applied. |
| `CHUNK_BYTES` | 48,000 (~48 KB) | Maximum size per chunk in `Chunk` mode. |
| `SUMMARY_CHARS` | 4,000 | Maximum preview length in `Summarize` mode. |

### Functions

- **`render_attachments(paths, policy)`** — Process a list of file paths, rendering each according to the policy. Returns a formatted string.
- **`AttachmentPolicy::parse(raw)`** — Parse from a string (`"reject"`, `"chunk"`, `"summarize"`).

## Public API

```rust
// Process multiple attachments
render_attachments(paths: &[PathBuf], policy: AttachmentPolicy) -> Result<String>

// Parse policy from string
AttachmentPolicy::parse(raw: &str) -> Result<AttachmentPolicy>
```

## Configuration

Set via the `--attachment-policy` CLI flag when using `praxis ask --file`:

```bash
praxis ask --file large-log.txt --attachment-policy chunk "Summarize this log"
praxis ask --file data.csv --attachment-policy summarize "What trends do you see?"
```

No `praxis.toml` fields are specific to this module.

## Usage

### CLI

```bash
# Small file — included verbatim (no policy needed)
praxis ask --file notes.md "What are the key action items?"

# Large file — chunk it
praxis ask --file big-report.txt --attachment-policy chunk "Analyze this"

# Large file — summarize it
praxis ask --file big-report.txt --attachment-policy summarize "Give me the highlights"

# Reject oversized files (default behavior)
praxis ask --file big-report.txt --attachment-policy reject "Read this"
# → Error: file exceeds 100000 bytes
```

### In code

```rust
use crate::attachments::{AttachmentPolicy, render_attachments};

let paths = vec![PathBuf::from("large-log.txt")];
let rendered = render_attachments(&paths, AttachmentPolicy::Chunk)?;
// rendered contains numbered chunks of the file content
```

### Summarize anchor extraction

The `summarize` policy extracts lines matching these patterns:

- Lines starting with `#` (Markdown headings)
- Lines starting with `- ` (list items)
- Lines containing `G-` (goal references)
- Lines containing `20` (likely dates)
- Lines containing "boundary" (case-insensitive)

Up to 12 anchor lines are included, followed by a 4,000-character preview of the file's beginning.

## Data Files

This module reads files provided by the operator. It does not write any files.

## Dependencies

No internal Praxis module dependencies. The module is self-contained, using only `std::fs` for file I/O.

## Source

`src/attachments.rs`
