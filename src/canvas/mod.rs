//! Live Canvas — streaming HTML workspace surface in the dashboard.
//!
//! #6 Live Canvas (from GAP_ANALYSIS_HERMES_OPENCLAW.md).
//!
//! Provides a lightweight streaming HTML surface that updates as the agent works.
//! The operator sees real-time visual feedback on what Praxis is doing —
//! without building a full Canvas protocol like OpenClaw.
//!
//! Architecture:
//! - Canvas state stored as HTML fragments in `data_dir/canvas/`
//! - Dashboard SSE endpoint pushes canvas updates
//! - Agent writes canvas "blocks" (text, code, status, progress)
//! - Operator sees rendered HTML in the dashboard

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A single block on the canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasBlock {
    /// Unique block ID.
    pub id: String,
    /// Block type (text, code, status, progress, image).
    pub kind: CanvasBlockKind,
    /// HTML content of the block.
    pub content: String,
    /// Block title/label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Order index for rendering.
    pub order: u32,
    /// Timestamp of last update.
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CanvasBlockKind {
    Text,
    Code,
    Status,
    Progress,
    Image,
}

/// The live canvas state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Canvas {
    /// Canvas blocks keyed by ID.
    blocks: HashMap<String, CanvasBlock>,
    /// Next order index.
    next_order: u32,
    /// Canvas creation timestamp.
    created_at: i64,
    /// Last update timestamp.
    updated_at: i64,
}

impl Canvas {
    /// Create a new canvas.
    pub fn new() -> Self {
        let now = now_secs();
        Self {
            blocks: HashMap::new(),
            next_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add or update a block on the canvas.
    pub fn set_block(
        &mut self,
        id: &str,
        kind: CanvasBlockKind,
        content: &str,
        title: Option<&str>,
    ) {
        let now = now_secs();
        let order = self.next_order;
        self.next_order += 1;

        self.blocks.insert(
            id.to_string(),
            CanvasBlock {
                id: id.to_string(),
                kind,
                content: content.to_string(),
                title: title.map(|s| s.to_string()),
                order,
                updated_at: now,
            },
        );
        self.updated_at = now;
    }

    /// Remove a block from the canvas.
    pub fn remove_block(&mut self, id: &str) -> bool {
        let removed = self.blocks.remove(id).is_some();
        if removed {
            self.updated_at = now_secs();
        }
        removed
    }

    /// Clear all blocks.
    pub fn clear(&mut self) {
        self.blocks.clear();
        self.next_order = 0;
        self.updated_at = now_secs();
    }

    /// Get all blocks sorted by order.
    pub fn blocks(&self) -> Vec<&CanvasBlock> {
        let mut blocks: Vec<_> = self.blocks.values().collect();
        blocks.sort_by_key(|b| b.order);
        blocks
    }

    /// Render the canvas as an HTML document.
    pub fn render_html(&self) -> String {
        let mut html = String::from("<div class=\"praxis-canvas\">\n");

        for block in self.blocks() {
            let title_html = block
                .title
                .as_deref()
                .map(|t| format!("<h3 class=\"canvas-block-title\">{t}</h3>"))
                .unwrap_or_default();

            let class = match block.kind {
                CanvasBlockKind::Text => "canvas-block-text",
                CanvasBlockKind::Code => "canvas-block-code",
                CanvasBlockKind::Status => "canvas-block-status",
                CanvasBlockKind::Progress => "canvas-block-progress",
                CanvasBlockKind::Image => "canvas-block-image",
            };

            html.push_str(&format!(
                "  <div class=\"canvas-block {class}\" id=\"{}\">\n    {title_html}\
                 <div class=\"canvas-block-content\">{}</div>\n  </div>\n",
                block.id, block.content
            ));
        }

        html.push_str("</div>");
        html
    }

    /// Number of blocks on the canvas.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

/// Canvas persistence manager.
pub struct CanvasStore {
    path: PathBuf,
}

impl CanvasStore {
    pub fn new(data_dir: &Path) -> Self {
        Self { path: data_dir.join("canvas") }
    }

    /// Save canvas state to disk.
    pub fn save(&self, canvas: &Canvas) -> Result<()> {
        fs::create_dir_all(&self.path)
            .with_context(|| format!("create {}", self.path.display()))?;

        let json = serde_json::to_string_pretty(canvas).context("serialize canvas")?;
        let file_path = self.path.join("canvas.json");
        fs::write(&file_path, json).with_context(|| format!("write {}", file_path.display()))?;

        // Also write rendered HTML for static serving
        let html_path = self.path.join("canvas.html");
        let html = canvas.render_html();
        fs::write(&html_path, html).with_context(|| format!("write {}", html_path.display()))?;

        Ok(())
    }

    /// Load canvas state from disk.
    pub fn load(&self) -> Result<Canvas> {
        let file_path = self.path.join("canvas.json");
        if !file_path.exists() {
            return Ok(Canvas::new());
        }
        let raw = fs::read_to_string(&file_path)?;
        let canvas: Canvas =
            serde_json::from_str(&raw).with_context(|| format!("parse {}", file_path.display()))?;
        Ok(canvas)
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_add_and_render() {
        let mut canvas = Canvas::new();
        canvas.set_block("status", CanvasBlockKind::Status, "Running task X", Some("Status"));
        canvas.set_block("code", CanvasBlockKind::Code, "fn main() {}", Some("Code"));

        let html = canvas.render_html();
        assert!(html.contains("Running task X"));
        assert!(html.contains("fn main()"));
        assert_eq!(canvas.len(), 2);
    }

    #[test]
    fn canvas_remove() {
        let mut canvas = Canvas::new();
        canvas.set_block("b1", CanvasBlockKind::Text, "hello", None);
        assert!(canvas.remove_block("b1"));
        assert!(canvas.is_empty());
    }
}
