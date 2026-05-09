//! Browser Automation via Chrome DevTools Protocol (CDP).
//!
//! #1 Browser Automation — connects to local Chrome via WebSocket CDP.
//!
//! Requires Chrome running with `--remote-debugging-port=9222`.
//! Tools: navigate, snapshot, vision, click, type, press, scroll, back, console.
//!
//! Usage: `praxis browser navigate https://example.com`
//! Check availability: `praxis browser status`

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde_json::Value;
use tungstenite::{Message, connect};

/// CDP WebSocket client — real WebSocket connection to Chrome.
pub struct CdpClient {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    id: std::sync::atomic::AtomicI64,
}

impl CdpClient {
    /// Connect to running Chrome instance.
    pub fn connect() -> Result<Self> {
        let port = Self::find_chrome_port();
        let debug_url = Self::get_debugger_url(port)?;
        let (socket, _) = connect(&debug_url as &str)
            .context("failed to connect to Chrome CDP — is Chrome running with --remote-debugging-port=9222?")?;
        Ok(Self {
            socket,
            id: std::sync::atomic::AtomicI64::new(0),
        })
    }

    /// Find which port Chrome is listening on.
    fn find_chrome_port() -> u16 {
        for port in [9222, 9223, 9224, 9225] {
            if std::net::TcpStream::connect(format!("localhost:{}", port)).is_ok() {
                return port;
            }
        }
        9222
    }

    /// Get the WebSocket debugger URL from Chrome's CDP endpoint.
    fn get_debugger_url(port: u16) -> Result<String> {
        let resp = reqwest::blocking::Client::new()
            .get(format!("http://localhost:{}/json", port))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .context(
                "failed to query Chrome CDP — is Chrome running with --remote-debugging-port=9222?",
            )?;

        let json: Value =
            serde_json::from_str(&resp.text().context("parse Chrome /json response")?)?;
        let ws_url = json
            .get("webSocketDebuggerUrl")
            .or_else(|| {
                json.as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("webSocketDebuggerUrl"))
            })
            .and_then(|v| v.as_str())
            .context("no webSocketDebuggerUrl in Chrome /json response")?;
        Ok(ws_url.to_string())
    }

    /// Send a CDP command and return the result.
    fn send(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let cmd = serde_json::json!({
            "id": id,
            "method": method,
            "params": params.unwrap_or(serde_json::Value::Null),
        });

        self.socket.send(Message::Text(cmd.to_string().into()))?;
        let resp = self.socket.read()?;
        let text = resp.into_text()?;

        // Chrome sends event messages too — skip until we get our id
        loop {
            let msg: Value = serde_json::from_str(&text)?;
            if let Some(msg_id) = msg.get("id").and_then(|v| v.as_i64())
                && msg_id == id
            {
                return Ok(msg.get("result").cloned().unwrap_or(serde_json::Value::Null));
            }
            // Wait for next message
            let resp = self.socket.read()?;
            let _text = resp.into_text()?;
        }
    }

    /// Enable Page domain before navigating.
    pub fn enable_page(&mut self) -> Result<()> {
        self.send("Page.enable", None)?;
        Ok(())
    }

    /// Navigate to a URL.
    pub fn navigate(&mut self, url: &str) -> Result<String> {
        let result = self.send("Page.navigate", Some(serde_json::json!({ "url": url })))?;
        if result.get("errorText").is_some() {
            bail!("navigation failed: {:?}", result);
        }
        let frame_id = result.get("frameId").and_then(|v| v.as_str()).unwrap_or("unknown");
        Ok(format!("Navigated to {} (frame: {})", url, frame_id))
    }

    /// Get page title.
    pub fn title(&mut self) -> Result<String> {
        let result = self.send(
            "Runtime.evaluate",
            Some(serde_json::json!({ "expression": "document.title" })),
        )?;
        let title = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("(no title)");
        Ok(title.to_string())
    }

    /// Get DOM snapshot as text.
    pub fn snapshot(&mut self) -> Result<String> {
        self.send("DOM.enable", None)?;
        let doc = self.send("DOM.getDocument", None)?;
        let node_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .unwrap_or(0);

        let snapshot = self.send(
            "DOM.getOuterHTML",
            Some(serde_json::json!({
                "nodeId": node_id,
                "depth": 3,
            })),
        )?;
        let html = snapshot.get("outerHTML").and_then(|v| v.as_str()).unwrap_or("(no content)");
        Ok(html.chars().take(500).collect::<String>())
    }

    /// Click element by CSS selector.
    pub fn click(&mut self, selector: &str) -> Result<String> {
        let escaped = selector.replace('\'', "\\'");
        let result = self.send("Runtime.evaluate", Some(serde_json::json!({
            "expression": format!(
                "(function() {{ const el = document.querySelector('{escaped}'); if (!el) return 'NOT_FOUND'; el.scrollIntoView(); el.click(); return 'CLICKED'; }})()"
            ),
        })))?;
        let status = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        Ok(format!("click {} → {}", selector, status))
    }

    /// Type into element by CSS selector.
    pub fn type_text(&mut self, selector: &str, text: &str) -> Result<String> {
        let escaped_sel = selector.replace('\'', "\\'");
        let escaped_text = text.replace('\'', "\\'").replace('\n', "\\n");
        let result = self.send("Runtime.evaluate", Some(serde_json::json!({
            "expression": format!(
                "(function() {{ const el = document.querySelector('{escaped_sel}'); if (!el) return 'NOT_FOUND'; el.focus(); el.value = ''; el.dispatchEvent(new Event('input')); el.value = '{escaped_text}'; el.dispatchEvent(new Event('input')); el.dispatchEvent(new Event('change')); return 'TYPED'; }})()"
            ),
        })))?;
        let status = result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        Ok(format!("type into {} → {}", selector, status))
    }

    /// Press a keyboard key.
    pub fn press(&mut self, key: &str) -> Result<String> {
        self.send(
            "Input.dispatchKeyEvent",
            Some(serde_json::json!({ "type": "keyDown", "key": key })),
        )?;
        self.send(
            "Input.dispatchKeyEvent",
            Some(serde_json::json!({ "type": "keyUp", "key": key })),
        )?;
        Ok(format!("pressed: {}", key))
    }

    /// Scroll page by pixel offset.
    pub fn scroll(&mut self, x: i64, y: i64) -> Result<String> {
        self.send(
            "Runtime.evaluate",
            Some(serde_json::json!({
                "expression": format!("window.scrollTo({}, {})", x, y),
            })),
        )?;
        Ok(format!("scrolled to ({}, {})", x, y))
    }

    /// Navigate back.
    pub fn back(&mut self) -> Result<String> {
        self.send("Runtime.goBack", None)?;
        Ok("navigated back".to_string())
    }

    /// Check if Chrome is available.
    /// Used by BrowserTool::is_available check.
    pub fn is_available() -> bool {
        std::net::TcpStream::connect("localhost:9222").is_ok()
    }
}

/// Browser automation tool entry point.
pub struct BrowserTool {
    client: Option<CdpClient>,
}

impl BrowserTool {
    pub fn new() -> Self {
        let mut client = CdpClient::connect().ok();
        if let Some(ref mut c) = client {
            let _ = c.enable_page();
        }
        Self { client }
    }
    pub fn is_available(&self) -> bool {
        self.client.is_some()
    }

    /// Execute a browser action.
    pub fn execute(&mut self, action: &str, params: HashMap<String, String>) -> Result<String> {
        let client = self.client.as_mut().context("Chrome not available")?;
        match action {
            "navigate" => {
                let url = params.get("url").context("navigate requires url")?;
                client.navigate(url)
            }
            "title" => client.title(),
            "snapshot" => client.snapshot(),
            "click" => {
                let selector = params.get("selector").context("click requires selector")?;
                client.click(selector)
            }
            "type" => {
                let selector = params.get("selector").context("type requires selector")?;
                let text = params.get("text").context("type requires text")?;
                client.type_text(selector, text)
            }
            "press" => {
                let key = params.get("key").context("press requires key")?;
                client.press(key)
            }
            "scroll" => {
                let x: i64 = params.get("x").and_then(|s| s.parse().ok()).unwrap_or(0);
                let y: i64 = params.get("y").and_then(|s| s.parse().ok()).unwrap_or(0);
                client.scroll(x, y)
            }
            "back" => client.back(),
            other => bail!("unknown browser action: {other}"),
        }
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry point called from execute.rs.
/// params: { "action": "navigate", "params": { "url": "https://..." } }
pub fn execute_browser_tool(params: &serde_json::Value) -> Result<String> {
    let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("snapshot");

    let mut tool = BrowserTool::new();
    if !tool.is_available() {
        bail!("Chrome not available — start Chrome with --remote-debugging-port=9222");
    }

    let mut args = HashMap::new();
    if let Some(obj) = params.get("params").and_then(|v| v.as_object()) {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                args.insert(k.clone(), s.to_string());
            }
        }
    }

    tool.execute(action, args)
}
