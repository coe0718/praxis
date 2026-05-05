use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

pub const MAX_ATTACHMENT_BYTES: usize = 100_000;
const CHUNK_BYTES: usize = 48_000;
const SUMMARY_CHARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentPolicy {
    Reject,
    Chunk,
    Summarize,
}

impl AttachmentPolicy {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "reject" => Ok(Self::Reject),
            "chunk" => Ok(Self::Chunk),
            "summarize" => Ok(Self::Summarize),
            _ => bail!("attachment policy must be one of reject, chunk, or summarize"),
        }
    }
}

pub fn render_attachments(paths: &[PathBuf], policy: AttachmentPolicy) -> Result<String> {
    let mut blocks = Vec::new();
    for path in paths {
        blocks.push(render_attachment(path, policy)?);
    }
    Ok(blocks.join("\n\n"))
}

fn render_attachment(path: &Path, policy: AttachmentPolicy) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let content = String::from_utf8(bytes.clone())
        .with_context(|| format!("{} is not valid UTF-8 text", path.display()))?;
    if bytes.len() <= MAX_ATTACHMENT_BYTES {
        return Ok(format!("Attachment {}:\n{}", path.display(), content));
    }

    match policy {
        AttachmentPolicy::Reject => bail!(
            "{} exceeds {} bytes; rerun with --attachment-policy chunk or summarize",
            path.display(),
            MAX_ATTACHMENT_BYTES
        ),
        AttachmentPolicy::Chunk => Ok(render_chunks(path, &content, bytes.len())),
        AttachmentPolicy::Summarize => Ok(render_summary(path, &content, bytes.len())),
    }
}

fn render_chunks(path: &Path, content: &str, byte_len: usize) -> String {
    let parts = chunk_text(content, CHUNK_BYTES);
    let total_parts = parts.len();
    let mut lines = vec![format!(
        "Attachment {} exceeded {} bytes ({} bytes); chunked into {} part(s).",
        path.display(),
        MAX_ATTACHMENT_BYTES,
        byte_len,
        total_parts
    )];
    for (index, part) in parts.into_iter().enumerate() {
        lines.push(format!("\n[chunk {}/{}]\n{}", index + 1, total_parts, part));
    }
    lines.join("\n")
}

fn render_summary(path: &Path, content: &str, byte_len: usize) -> String {
    let mut lines = vec![format!(
        "Attachment {} exceeded {} bytes ({} bytes); summarized instead of truncating.",
        path.display(),
        MAX_ATTACHMENT_BYTES,
        byte_len
    )];
    let selected = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            line.starts_with('#')
                || line.starts_with("- ")
                || line.contains("G-")
                || line.contains("20")
                || line.to_ascii_lowercase().contains("boundary")
        })
        .take(12)
        .collect::<Vec<_>>();
    if !selected.is_empty() {
        lines.push("Anchors:".to_string());
        lines.extend(selected.into_iter().map(|line| format!("- {line}")));
    }
    let preview = truncate_chars(content, SUMMARY_CHARS);
    lines.push("Preview:".to_string());
    lines.push(preview);
    lines.join("\n")
}

fn chunk_text(content: &str, max_bytes: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_bytes = 0;

    for ch in content.chars() {
        let len = ch.len_utf8();
        if !current.is_empty() && current_bytes + len > max_bytes {
            chunks.push(current);
            current = String::new();
            current_bytes = 0;
        }
        current.push(ch);
        current_bytes += len;
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    chunks
}

fn truncate_chars(content: &str, limit: usize) -> String {
    if content.chars().count() <= limit {
        return content.to_string();
    }
    let shortened = content.chars().take(limit.saturating_sub(3)).collect::<String>();
    format!("{}...", shortened.trim_end())
}

#[cfg(test)]
mod tests {
    use super::{AttachmentPolicy, chunk_text};

    #[test]
    fn parses_policy_values() {
        assert_eq!(AttachmentPolicy::parse("chunk").unwrap(), AttachmentPolicy::Chunk);
        assert!(AttachmentPolicy::parse("bogus").is_err());
    }

    #[test]
    fn chunker_preserves_all_content() {
        let content = "a".repeat(100_500);
        let chunks = chunk_text(&content, 48_000);
        assert_eq!(chunks.concat(), content);
        assert!(chunks.len() > 1);
    }
}
