# Skills

> Progressive skill loading, catalog rendering, and automatic skill synthesis from successful sessions.

## Overview

The `skills` module manages Praxis's reusable workflow library. Skills are Markdown files with TOML frontmatter that live in the `skills/` directory. Each skill describes a repeatable workflow — for example, "morning brief delivery" or "code review" — that the agent can consult during the Orient phase.

To keep context costs low, Orient loads only a compact catalog (name, description, tags, token estimate). Full skill content is loaded on-demand when the agent decides a skill is relevant. This two-tier loading strategy means a library of 20+ skills adds minimal overhead to each session.

The module also includes an automatic **skill synthesizer** that drafts new skills from sufficiently complex, successful sessions. Drafts are written to `skills/drafts/` and are never activated automatically — they await operator review.

## Architecture

### Types (`mod.rs`)

| Type | Description |
|------|-------------|
| `SkillEntry` | Compact catalog metadata: `id` (from filename stem), `name`, `description`, `tags`, `token_estimate`. |
| `SkillFrontmatter` | Internal TOML frontmatter struct parsed from `+++` delimiters. |

### Catalog Functions

| Function | Description |
|----------|-------------|
| `load_catalog()` | Reads all `*.md` files from `skills_dir`, parses frontmatter, returns sorted `Vec<SkillEntry>`. Malformed files are silently skipped. |
| `render_catalog()` | Renders the catalog as a Markdown section suitable for context injection. Returns empty string when no skills are installed. |
| `read_skill_content()` | Loads the full body of a skill by ID, stripping the TOML frontmatter. |

### Skill Synthesis (`synthesis.rs`)

| Type | Description |
|------|-------------|
| `SkillSynthesisInput` | Session data used to decide whether to draft: outcome, goal title, tool call count, action summary, session end time. |
| `SkillSynthesizer` | Evaluates session complexity and writes draft `SKILL.md` files when thresholds are met. |

**Drafting criteria:**
1. Session outcome must be a success variant (`completed`, `success`, `done`, `reviewer_passed`).
2. A named goal must have been worked on (not idle maintenance).
3. At least 3 tool calls were made (proxy for multi-step complexity).

Drafts include inferred tags (from keyword matching), a session summary, and placeholder step sections for the operator to fill in.

## Public API

```rust
// Catalog
pub fn load_catalog(skills_dir: &Path) -> Vec<SkillEntry>;
pub fn render_catalog(skills_dir: &Path) -> String;
pub fn read_skill_content(skills_dir: &Path, skill_id: &str) -> Option<String>;

// Synthesis
pub struct SkillSynthesizer;
impl SkillSynthesizer {
    pub fn maybe_draft(&self, drafts_dir: &Path, input: &SkillSynthesisInput<'_>) -> Result<Option<PathBuf>>;
}
```

## Configuration

No `praxis.toml` section is required. Skills are loaded from `data_dir/skills/` by default. The default token estimate for skills without an explicit `token_estimate` in frontmatter is 500.

### Skill File Format

```markdown
+++
name = "morning-brief"
description = "Collect and push a morning brief to Telegram"
tags = ["telegram", "brief", "daily"]
token_estimate = 800
+++

## Steps
1. Gather memory and goal summaries
2. Format the brief
3. Push via Telegram
```

## Usage

Skills are loaded automatically by the Orient phase. The CLI provides:

```bash
# List installed skills
praxis tools list

# Skill drafts appear in:
ls data_dir/skills/drafts/
```

To activate a draft, review it, edit the steps, and move it from `skills/drafts/` to `skills/`.

## Data Files

| File/Directory | Format | Purpose |
|----------------|--------|---------|
| `skills/*.md` | Markdown + TOML frontmatter | Installed skills loaded during Orient. |
| `skills/drafts/*.md` | Markdown + TOML frontmatter | Auto-drafted skills awaiting operator review. |

## Dependencies

- `paths` — `PraxisPaths` for skill directory locations
- `toml` — frontmatter parsing
- `serde`, `chrono`, `anyhow`

## Source

`src/skills/` — `mod.rs`, `synthesis.rs`
