# Skill Pack

> Skill packs — bundled collections with manifest. Skills can be packaged as reusable collections with standardized manifests and dependency resolution.

## Overview

The `skill_pack` module provides a packaging system for distributing skills as reusable bundles. A `SkillPack` is a directory containing a `pack.toml` manifest file plus skill entry files. The `SkillPackManifest` defines the pack identity (id, version, description, author), the list of included `SkillInfo` entries (each with an id, entry point file, required capabilities, and optional config schema), dependencies on other packs, and compatibility tags.

The `SkillPackRegistry` tracks installed packs on disk, supporting load/save from a TOML file, adding new packs, and listing installed pack IDs.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `SkillPackManifest` | Pack metadata: `id`, `version`, `description`, `author`, `skills`, `dependencies`, `tags`. |
| `SkillInfo` | Single skill entry: `id`, `entry` (file path), `requires` (capabilities), `config_schema` (optional JSON schema). |
| `SkillPack` | A loaded pack: `manifest` + `path` (filesystem location). |
| `SkillPackRegistry` | Registry of installed packs, persisted as TOML. |

### Pack Loading and Installation

1. `SkillPack::load(path)` reads `pack.toml` from the directory, deserializes it to `SkillPackManifest`.
2. `SkillPack::install(target_dir)` copies skill entry files to `target_dir/<pack_id>/<entry>`.
3. `SkillPack::validate_dependencies(installed)` returns a list of missing dependency pack IDs.

## Public API

```rust
// Manifest
pub struct SkillPackManifest {
    pub id: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub skills: Vec<SkillInfo>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
}

// Skill entry in a pack
pub struct SkillInfo {
    pub id: String,
    pub entry: String,
    pub requires: Vec<String>,
    pub config_schema: Option<serde_json::Value>,
}

// Loaded pack
pub struct SkillPack {
    pub manifest: SkillPackManifest,
    pub path: PathBuf,
}
impl SkillPack {
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error>;
    pub fn install(&self, target_dir: &Path) -> Result<(), anyhow::Error>;
    pub fn validate_dependencies(&self, installed: &[&str]) -> Vec<String>;
}

// Registry
pub struct SkillPackRegistry {
    pub packs: HashMap<String, SkillPackManifest>,
}
impl SkillPackRegistry {
    pub fn load(path: &Path) -> Result<Self, anyhow::Error>;
    pub fn save(&self, path: &Path) -> Result<(), anyhow::Error>;
    pub fn add(&mut self, manifest: SkillPackManifest);
    pub fn installed_ids(&self) -> Vec<&str>;
}
```

## Configuration

### pack.toml Format

```toml
id = "devops-toolkit"
version = "1.0.0"
description = "Common DevOps automation skills"
author = "Axonix Team"
dependencies = ["core-utils"]
tags = ["devops", "automation", "ci"]

[[skills]]
id = "deploy-check"
entry = "deploy_check.skill.md"
requires = ["docker", "git"]
config_schema = { type = "object", properties = { timeout = { type = "integer" } } }
```

### Example: Loading and Installing a Pack

```rust
let pack = SkillPack::load(PathBuf::from("packs/devops-toolkit"))?;

// Validate dependencies
let installed = registry.installed_ids();
let missing = pack.validate_dependencies(&installed);
if !missing.is_empty() {
    eprintln!("Missing packs: {:?}", missing);
}

// Install to skills directory
pack.install(Path::new("skills/"))?;

// Register
registry.add(pack.manifest);
registry.save(Path::new("skills/packs.toml"))?;
```

## Data Files

| File | Format | Purpose |
|------|--------|---------|
| `<pack_dir>/pack.toml` | TOML | Skill pack manifest |
| `<pack_dir>/<entry>` | Skill Markdown | Individual skill file |
| `skills/packs.toml` | TOML | Registry of installed packs |

## Dependencies

- `serde` / `serde_json` — serialization
- `toml` — manifest parsing
- `anyhow` — error handling
- `std::path` — filesystem operations

## Source

`src/skill_pack.rs`