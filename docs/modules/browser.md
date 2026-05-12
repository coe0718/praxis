# Browser

> Browser automation — safe web tasks via Docker containers. Provides browser automation in isolated Docker containers for safe web scraping and automation without exposing the host.

## Overview

The `browser` module provides a lightweight interface for browser automation. It manages `BrowserSession` instances with a unique session ID, optional Docker container ID, current URL tracking, and viewport configuration. The module exposes three free functions: `create_browser_session`, `execute_browser_task`, and `close_browser_session`.

Currently the implementation is a scaffold — `create_browser_session` creates a `BrowserSession` record with a unique ID and default viewport (1280×800), `execute_browser_task` returns an error indicating the feature is not fully implemented, and `close_browser_session` is a no-op. The module is designed to integrate with Docker-based browser containers (e.g., Playwright or Puppeteer in containers) for full isolation.

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `BrowserSession` | Session state: `id`, `container_id` (optional), `current_url` (optional), `viewport`. |
| `Viewport` | Browser viewport dimensions: `width`, `height` (default: 1280×800). |

## Public API

```rust
// Browser session
pub struct BrowserSession {
    pub id: String,
    pub container_id: Option<String>,
    pub current_url: Option<String>,
    pub viewport: Viewport,
}

pub struct Viewport {
    pub width: u32,
    pub height: u32,
}
impl Default for Viewport;

// Free functions
pub fn create_browser_session(_paths: &PraxisPaths, _url: Option<&str>) -> Result<BrowserSession>;
pub async fn execute_browser_task(_session: &BrowserSession, _script: &str) -> Result<String>;
pub async fn close_browser_session(_session: &BrowserSession) -> Result<()>;
```

## Configuration

No `praxis.toml` section. Sessions are created programmatically.

### Example

```rust
use crate::paths::PraxisPaths;

let paths = PraxisPaths::load().unwrap();
let session = create_browser_session(&paths, Some("https://example.com"))?;
println!("Session {} created with viewport {}x{}",
    session.id, session.viewport.width, session.viewport.height);

// Future: execute_browser_task(&session, "document.title").await?;
close_browser_session(&session).await?;
```

## Dependencies

- `anyhow` — error handling
- `chrono` — session ID timestamp generation
- `paths` — `PraxisPaths` for file locations

## Source

`src/browser.rs`