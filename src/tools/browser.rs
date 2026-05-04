//! Browser automation tool — headless Chromium or HTTP fetch fallback.
//!
//! Provides navigate, click, snapshot, type, scroll, screenshot operations.
//! Uses headless Chromium via subprocess when available, falls back to curl.

use std::{
    fs,
    path::{Path},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Browser session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSession {
    pub current_url: String,
    pub history: Vec<String>,
    pub title: String,
    pub page_text: String,
}

impl Default for BrowserSession {
    fn default() -> Self {
        Self {
            current_url: String::new(),
            history: Vec::new(),
            title: String::new(),
            page_text: String::new(),
        }
    }
}

/// Result of a browser operation.
#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserResult {
    pub action: String,
    pub url: String,
    pub title: String,
    pub text: String,
    pub html: Option<String>,
    pub screenshot_path: Option<String>,
    pub console_output: Option<String>,
    pub error: Option<String>,
}

/// Navigate to a URL and return page content.
fn navigate(url: &str) -> Result<BrowserResult> {
    // Try headless chromium first
    if has_chromium() {
        navigate_chromium(url)
    } else {
        navigate_curl(url)
    }
}

fn has_chromium() -> bool {
    for cmd in &["chromium-browser", "chromium", "google-chrome", "chrome"] {
        if Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn navigate_chromium(url: &str) -> Result<BrowserResult> {
    let output = Command::new(find_chromium())
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--dump-dom",
            "--virtual-time-budget=5000",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("running headless chromium")?;

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let text = html_to_text(&html);

    Ok(BrowserResult {
        action: "navigate".to_string(),
        url: url.to_string(),
        title: extract_title(&html),
        text,
        html: Some(html),
        screenshot_path: None,
        console_output: if stderr.is_empty() { None } else { Some(stderr) },
        error: None,
    })
}

fn navigate_curl(url: &str) -> Result<BrowserResult> {
    let output = Command::new("curl")
        .args(["-sL", "--max-time", "15", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("running curl")?;

    let html = String::from_utf8_lossy(&output.stdout).to_string();
    let text = html_to_text(&html);

    Ok(BrowserResult {
        action: "navigate (curl fallback)".to_string(),
        url: url.to_string(),
        title: extract_title(&html),
        text,
        html: Some(html),
        screenshot_path: None,
        console_output: None,
        error: None,
    })
}

fn find_chromium() -> &'static str {
    for cmd in &["chromium-browser", "chromium", "google-chrome", "chrome"] {
        if Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return cmd;
        }
    }
    "chromium-browser"
}

/// Take a screenshot (via chromium --screenshot).
fn screenshot(url: &str, output_path: &Path) -> Result<BrowserResult> {
    if !has_chromium() {
        // Fallback: just fetch the page
        let result = navigate_curl(url)?;
        return Ok(BrowserResult {
            action: "screenshot (no browser — text-only fallback)".to_string(),
            ..result
        });
    }

    let parent = output_path.parent().unwrap_or(Path::new("."));
    let _ = fs::create_dir_all(parent);

    let _output = Command::new(find_chromium())
        .args([
            "--headless",
            "--disable-gpu",
            "--no-sandbox",
            "--screenshot",
            output_path.to_str().unwrap_or("screenshot.png"),
            "--window-size=1280,720",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("taking screenshot")?;

    let exists = output_path.exists();
    Ok(BrowserResult {
        action: "screenshot".to_string(),
        url: url.to_string(),
        title: String::new(),
        text: String::new(),
        html: None,
        screenshot_path: if exists {
            Some(output_path.display().to_string())
        } else {
            None
        },
        console_output: None,
        error: if exists {
            None
        } else {
            Some("screenshot file not created".to_string())
        },
    })
}

/// Simple HTML-to-text conversion (strips tags).
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();
    // Remove scripts and styles
    text = regex_lite_replace(&text);
    // Collapse whitespace
    let mut result = String::new();
    let mut last_was_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }
    result.trim().chars().take(10_000).collect()
}

fn regex_lite_replace(html: &str) -> String {
    // Simple tag stripping without regex
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let lower = html.to_lowercase();
    let bytes = html.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        if lower[i..].starts_with("<script") {
            in_script = true;
            i += 7;
            continue;
        }
        if in_script && lower[i..].starts_with("</script>") {
            in_script = false;
            i += 9;
            continue;
        }
        if in_script {
            i += 1;
            continue;
        }
        if lower[i..].starts_with("<style") {
            // Skip to </style>
            if let Some(end) = lower[i..].find("</style>") {
                i += end + 8;
                continue;
            }
        }
        if bytes[i] == b'<' {
            in_tag = true;
            i += 1;
            continue;
        }
        if in_tag && bytes[i] == b'>' {
            in_tag = false;
            i += 1;
            continue;
        }
        if in_tag {
            i += 1;
            continue;
        }
        // Decode common entities
        if lower[i..].starts_with("&nbsp;") {
            result.push(' ');
            i += 6;
        } else if lower[i..].starts_with("&amp;") {
            result.push('&');
            i += 5;
        } else if lower[i..].starts_with("&lt;") {
            result.push('<');
            i += 4;
        } else if lower[i..].starts_with("&gt;") {
            result.push('>');
            i += 4;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    // Add newlines for block elements
    result
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<p>", "\n")
        .replace("</p>", "\n")
        .replace("<div>", "\n")
        .replace("</div>", "\n")
        .replace("<h1>", "\n# ")
        .replace("<h2>", "\n## ")
        .replace("<h3>", "\n### ")
}

fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        let content_start = start + 7;
        if let Some(end) = lower[content_start..].find("</title>") {
            return html[content_start..content_start + end].trim().to_string();
        }
    }
    String::new()
}

/// Execute the browser tool from a tool call.
pub fn execute_browser_tool(params: &serde_json::Value) -> Result<String> {
    let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("navigate");

    match action {
        "navigate" | "goto" | "get" => {
            let url = params
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'url' parameter"))?;
            let result = navigate(url)?;
            Ok(format!(
                "## {}\nURL: {}\n\n{}",
                if result.title.is_empty() {
                    "(no title)"
                } else {
                    &result.title
                },
                result.url,
                result.text.chars().take(3000).collect::<String>()
            ))
        }
        "screenshot" => {
            let url = params
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'url' parameter"))?;
            let output = params
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/praxis-screenshot.png");
            let result = screenshot(url, Path::new(output))?;
            match &result.screenshot_path {
                Some(path) => Ok(format!("Screenshot saved: {}", path)),
                None => Ok(format!("Screenshot failed: {}", result.error.unwrap_or_default())),
            }
        }
        "snapshot" | "text" => {
            let url = params
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("missing 'url' parameter"))?;
            let result = navigate(url)?;
            Ok(result.text)
        }
        "click" | "type" | "scroll" | "press_key" => Ok(format!(
            "Action '{}' requires interactive browser (not yet supported in headless mode). Use navigate to fetch page content.",
            action
        )),
        _ => bail!(
            "unknown browser action: '{}'. Supported: navigate, screenshot, snapshot",
            action
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_text() {
        let html =
            "<html><head><title>Test</title></head><body><p>Hello <b>world</b></p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>My Page</title></head><body></body></html>";
        assert_eq!(extract_title(html), "My Page");
    }

    #[test]
    fn test_strip_scripts() {
        let html = "<body>Before<script>alert('xss')</script>After</body>";
        let text = html_to_text(html);
        assert!(text.contains("Before"));
        assert!(text.contains("After"));
        assert!(!text.contains("alert"));
    }
}
