use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

pub(super) fn ensure_no_overlap(left: &Path, right: &Path, label: &str) -> Result<()> {
    let left = absoluteish(left)?;
    let right = absoluteish(right)?;
    if left.starts_with(&right) || right.starts_with(&left) {
        bail!("{label} must not overlap {}", right.display());
    }
    Ok(())
}

pub(super) fn prepare_fresh_dir(path: &Path, overwrite: bool) -> Result<()> {
    if path.exists() {
        if !overwrite {
            bail!("{} already exists; pass --overwrite to replace it", path.display());
        }
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    Ok(())
}

pub(super) fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(src)
        .with_context(|| format!("failed to inspect {}", src.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("refusing to copy symlink {}", src.display());
    }
    if !metadata.is_file() {
        bail!("expected file {}", src.display());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(src, dst)
        .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display()))?;
    Ok(())
}

pub(super) fn copy_dir_tree(
    src_root: &Path,
    dst_root: &Path,
    skip: &[PathBuf],
) -> Result<Vec<String>> {
    let skip = skip.iter().map(|path| absoluteish(path)).collect::<Result<Vec<_>>>()?;
    let mut copied = Vec::new();
    visit(src_root, src_root, dst_root, &skip, &mut copied)?;
    copied.sort();
    Ok(copied)
}

pub(super) fn restore_dir_tree(
    src_root: &Path,
    dst_root: &Path,
    files: &[String],
) -> Result<usize> {
    for raw in files {
        let relative = safe_relative(raw)?;
        copy_file(&src_root.join(&relative), &dst_root.join(&relative))?;
    }
    Ok(files.len())
}

pub(super) fn safe_relative(raw: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            _ => bail!("unsafe relative path {raw}"),
        }
    }
    Ok(normalized)
}

fn visit(
    root: &Path,
    current: &Path,
    dst_root: &Path,
    skip: &[PathBuf],
    copied: &mut Vec<String>,
) -> Result<()> {
    for entry in
        fs::read_dir(current).with_context(|| format!("failed to read {}", current.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", current.display()))?;
        let path = entry.path();
        let absolute = absoluteish(&path)?;
        if skip.iter().any(|candidate| candidate == &absolute) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!("refusing to copy symlink {}", path.display());
        }
        if metadata.is_dir() {
            visit(root, &path, dst_root, skip, copied)?;
            continue;
        }
        if !metadata.is_file() {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed to strip {}", root.display()))?;
        copy_file(&path, &dst_root.join(relative))?;
        copied.push(relative.to_string_lossy().replace('\\', "/"));
    }
    Ok(())
}

fn absoluteish(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir().context("failed to resolve current directory")?.join(path))
    }
}
