use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::paths::PraxisPaths;

use super::{ToolKind, ToolManifest};

pub trait ToolRegistry {
    fn ensure_foundation(&self, paths: &PraxisPaths) -> Result<()>;
    fn validate(&self, paths: &PraxisPaths) -> Result<()>;
    fn list(&self, paths: &PraxisPaths) -> Result<Vec<ToolManifest>>;
    fn get(&self, paths: &PraxisPaths, name: &str) -> Result<Option<ToolManifest>>;
    fn register(&self, paths: &PraxisPaths, manifest: &ToolManifest) -> Result<PathBuf>;
    fn summary(&self, paths: &PraxisPaths) -> Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct FileToolRegistry;

impl ToolRegistry for FileToolRegistry {
    fn ensure_foundation(&self, paths: &PraxisPaths) -> Result<()> {
        fs::create_dir_all(&paths.tools_dir)
            .with_context(|| format!("failed to create {}", paths.tools_dir.display()))?;

        for manifest in default_manifests() {
            let path = manifest_path(&paths.tools_dir, &manifest.name);
            if !path.exists() {
                self.register(paths, &manifest)?;
            }
        }

        Ok(())
    }

    fn validate(&self, paths: &PraxisPaths) -> Result<()> {
        let mut names = HashSet::new();
        for manifest in self.list(paths)? {
            manifest.validate()?;
            if !names.insert(manifest.name.clone()) {
                bail!("duplicate tool manifest {}", manifest.name);
            }
        }
        Ok(())
    }

    fn list(&self, paths: &PraxisPaths) -> Result<Vec<ToolManifest>> {
        if !paths.tools_dir.exists() {
            return Ok(Vec::new());
        }

        let mut manifests = Vec::new();
        for entry in fs::read_dir(&paths.tools_dir)
            .with_context(|| format!("failed to read {}", paths.tools_dir.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read {}", paths.tools_dir.display()))?;
            if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                manifests.push(load_manifest(&entry.path())?);
            }
        }

        manifests.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(manifests)
    }

    fn get(&self, paths: &PraxisPaths, name: &str) -> Result<Option<ToolManifest>> {
        Ok(self
            .list(paths)?
            .into_iter()
            .find(|manifest| manifest.name == name))
    }

    fn register(&self, paths: &PraxisPaths, manifest: &ToolManifest) -> Result<PathBuf> {
        manifest.validate()?;
        fs::create_dir_all(&paths.tools_dir)
            .with_context(|| format!("failed to create {}", paths.tools_dir.display()))?;
        let path = manifest_path(&paths.tools_dir, &manifest.name);
        let raw = toml::to_string_pretty(manifest).context("failed to serialize tool manifest")?;
        fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    fn summary(&self, paths: &PraxisPaths) -> Result<String> {
        let manifests = self.list(paths)?;
        if manifests.is_empty() {
            return Ok("No registered tools.".to_string());
        }

        Ok(manifests
            .into_iter()
            .map(|manifest| format!("- {}", manifest.summary_line()))
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

fn load_manifest(path: &Path) -> Result<ToolManifest> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let manifest = toml::from_str::<ToolManifest>(&raw)
        .with_context(|| format!("invalid TOML in {}", path.display()))?;
    manifest.validate()?;
    Ok(manifest)
}

fn manifest_path(root: &Path, name: &str) -> PathBuf {
    let slug = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => ch.to_ascii_lowercase(),
            _ => '-',
        })
        .collect::<String>();
    root.join(format!("{slug}.toml"))
}

fn default_manifests() -> Vec<ToolManifest> {
    vec![
        ToolManifest {
            name: "internal-maintenance".to_string(),
            description: "Safe internal bookkeeping with no external side effects.".to_string(),
            kind: ToolKind::Internal,
            required_level: 1,
            requires_approval: false,
            rehearsal_required: false,
            allowed_paths: Vec::new(),
        },
        ToolManifest {
            name: "praxis-data-write".to_string(),
            description: "Controlled writes inside the Praxis data directory.".to_string(),
            kind: ToolKind::Shell,
            required_level: 2,
            requires_approval: true,
            rehearsal_required: true,
            allowed_paths: vec![
                "JOURNAL.md".to_string(),
                "LEARNINGS.md".to_string(),
                "METRICS.md".to_string(),
                "PROPOSALS.md".to_string(),
            ],
        },
    ]
}
