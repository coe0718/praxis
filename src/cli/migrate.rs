//! Migration tooling — import from Axonix and other agent frameworks.
//!
//! `praxis migrate axonix <path>` reads an Axonix data directory and imports
//! SOUL.md, memories, skills, command allowlists, and API key references
//! into the Praxis data directory.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::paths::{PraxisPaths, default_data_dir};

/// Migration report — summary of what was imported.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MigrationReport {
    pub source: String,
    pub items: Vec<MigrationItem>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationItem {
    pub kind: String,
    pub source_path: String,
    pub dest_path: String,
    pub bytes: u64,
}

/// Migrate from an Axonix data directory.
pub fn migrate_from_axonix(
    paths: &PraxisPaths,
    source_dir: &Path,
    dry_run: bool,
) -> Result<MigrationReport> {
    if !source_dir.exists() {
        bail!("source directory does not exist: {}", source_dir.display());
    }

    let mut report = MigrationReport {
        source: format!("axonix:{}", source_dir.display()),
        ..Default::default()
    };

    // 1. SOUL.md → SOUL.md
    migrate_file(&source_dir.join("SOUL.md"), &paths.soul_file, "soul", dry_run, &mut report);

    // 2. Axonix IDENTITY.md → Praxis IDENTITY.md
    migrate_file(
        &source_dir.join("IDENTITY.md"),
        &paths.identity_file,
        "identity",
        dry_run,
        &mut report,
    );

    // 3. GOALS.md → GOALS.md
    migrate_file(&source_dir.join("GOALS.md"), &paths.goals_file, "goals", dry_run, &mut report);

    // 4. AGENTS.md → AGENTS.md
    migrate_file(
        &source_dir.join("AGENTS.md"),
        &paths.agents_file,
        "agents",
        dry_run,
        &mut report,
    );

    // 5. ROADMAP.md → ROADMAP.md
    migrate_file(
        &source_dir.join("ROADMAP.md"),
        &paths.roadmap_file,
        "roadmap",
        dry_run,
        &mut report,
    );

    // 6. Skills directory → skills/
    let source_skills = source_dir.join("skills");
    if source_skills.exists() {
        migrate_directory(&source_skills, &paths.skills_dir, "skill", dry_run, &mut report);
    } else {
        report.skipped.push("skills/ (not found in source)".to_string());
    }

    // 7. Tools directory → tools/
    let source_tools = source_dir.join("tools");
    if source_tools.exists() {
        migrate_directory(&source_tools, &paths.tools_dir, "tool", dry_run, &mut report);
    } else {
        report.skipped.push("tools/ (not found in source)".to_string());
    }

    // 8. User memory → user_memory.json
    let source_memory = source_dir.join("user_memory.json");
    if source_memory.exists() {
        migrate_file(&source_memory, &paths.user_memory_file, "memory", dry_run, &mut report);
    } else {
        report.skipped.push("user_memory.json (not found)".to_string());
    }

    // 9. Config → extract provider info as a reference
    let source_config = source_dir.join("axonix.toml");
    if source_config.exists() {
        report.skipped.push(
            "axonix.toml: config format differs — provider keys need manual migration".to_string(),
        );
    }

    // 10. Telegram state
    let source_telegram = source_dir.join("telegram_state.json");
    if source_telegram.exists() {
        migrate_file(
            &source_telegram,
            &paths.telegram_state_file,
            "telegram_state",
            dry_run,
            &mut report,
        );
    }

    // 11. Hooks
    let source_hooks = source_dir.join("hooks.toml");
    if source_hooks.exists() {
        migrate_file(&source_hooks, &paths.hooks_file, "hooks", dry_run, &mut report);
    }

    Ok(report)
}

fn migrate_file(
    source: &Path,
    dest: &Path,
    kind: &str,
    dry_run: bool,
    report: &mut MigrationReport,
) {
    if !source.exists() {
        report.skipped.push(format!("{} (not found)", source.display()));
        return;
    }

    if dest.exists() && !dry_run {
        report.skipped.push(format!("{} (already exists, skipping)", dest.display()));
        return;
    }

    let bytes = match fs::metadata(source) {
        Ok(m) => m.len(),
        Err(e) => {
            report.errors.push(format!("{}: {e}", source.display()));
            return;
        }
    };

    if !dry_run {
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::copy(source, dest) {
            report
                .errors
                .push(format!("copy {} → {}: {e}", source.display(), dest.display()));
            return;
        }
    }

    report.items.push(MigrationItem {
        kind: kind.to_string(),
        source_path: source.display().to_string(),
        dest_path: dest.display().to_string(),
        bytes,
    });
}

fn migrate_directory(
    source: &Path,
    dest: &Path,
    kind: &str,
    dry_run: bool,
    report: &mut MigrationReport,
) {
    if !source.exists() {
        return;
    }

    let entries = match fs::read_dir(source) {
        Ok(e) => e,
        Err(e) => {
            report.errors.push(format!("read_dir {}: {e}", source.display()));
            return;
        }
    };

    for entry in entries.flatten() {
        let src_path = entry.path();
        if src_path.is_dir() {
            // Recurse into subdirectories
            let dest_sub = dest.join(entry.file_name());
            migrate_directory(&src_path, &dest_sub, kind, dry_run, report);
        } else {
            let dest_path = dest.join(entry.file_name());
            migrate_file(&src_path, &dest_path, kind, dry_run, report);
        }
    }
}

/// CLI handler for `praxis migrate`.
pub fn handle_migrate(data_dir: Option<PathBuf>, source: PathBuf, dry_run: bool) -> Result<String> {
    let data_dir = data_dir.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    let source_dir = source
        .canonicalize()
        .with_context(|| format!("cannot resolve source path: {}", source.display()))?;

    let report = migrate_from_axonix(&paths, &source_dir, dry_run)?;

    let mut lines = Vec::new();

    if dry_run {
        lines.push(format!("DRY RUN — migration from {}", report.source));
    } else {
        lines.push(format!("Migration from {}", report.source));
    }

    lines.push(format!("  Imported: {} items", report.items.len()));
    for item in &report.items {
        lines.push(format!(
            "    [{}] {} → {} ({:.1} KB)",
            item.kind,
            Path::new(&item.source_path).file_name().unwrap_or_default().to_string_lossy(),
            Path::new(&item.dest_path).file_name().unwrap_or_default().to_string_lossy(),
            item.bytes as f64 / 1024.0,
        ));
    }

    if !report.skipped.is_empty() {
        lines.push(format!("  Skipped: {} items", report.skipped.len()));
        for skip in &report.skipped {
            lines.push(format!("    - {skip}"));
        }
    }

    if !report.errors.is_empty() {
        lines.push(format!("  Errors: {}", report.errors.len()));
        for err in &report.errors {
            lines.push(format!("    ! {err}"));
        }
    }

    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_paths() -> (PraxisPaths, tempfile::TempDir, tempfile::TempDir) {
        let dest_tmp = tempdir().unwrap();
        let source_tmp = tempdir().unwrap();
        let paths = PraxisPaths::for_data_dir(dest_tmp.path().to_path_buf());
        fs::create_dir_all(&paths.data_dir).unwrap();
        (paths, dest_tmp, source_tmp)
    }

    #[test]
    fn test_migrate_axonix_soul() {
        let (paths, _dest, source) = setup_paths();

        // Create source SOUL.md
        fs::write(source.path().join("SOUL.md"), "# I am Axonix").unwrap();

        let report = migrate_from_axonix(&paths, source.path(), false).unwrap();
        assert!(report.items.iter().any(|i| i.kind == "soul"));

        let content = fs::read_to_string(&paths.soul_file).unwrap();
        assert!(content.contains("Axonix"));
    }

    #[test]
    fn test_migrate_dry_run() {
        let (paths, _dest, source) = setup_paths();
        fs::write(source.path().join("SOUL.md"), "# Test").unwrap();

        let report = migrate_from_axonix(&paths, source.path(), true).unwrap();
        assert!(report.items.iter().any(|i| i.kind == "soul"));
        // File should NOT exist in dry run
        assert!(!paths.soul_file.exists());
    }

    #[test]
    fn test_migrate_skills_directory() {
        let (paths, _dest, source) = setup_paths();
        let skills_dir = source.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("coding.md"), "# Coding Skill").unwrap();
        fs::write(skills_dir.join("review.md"), "# Review Skill").unwrap();

        let report = migrate_from_axonix(&paths, source.path(), false).unwrap();
        let skill_items: Vec<_> = report.items.iter().filter(|i| i.kind == "skill").collect();
        assert_eq!(skill_items.len(), 2);

        assert!(paths.skills_dir.join("coding.md").exists());
        assert!(paths.skills_dir.join("review.md").exists());
    }
}
