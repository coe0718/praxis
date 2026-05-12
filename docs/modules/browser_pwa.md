# Browser PWA

> Browser-only PWA mode — agent runs entirely in a browser tab. Compile Praxis agent to WebAssembly for client-side execution with state synced via cloud storage and identity via WebCrypto.

## Overview

The `browser_pwa` module enables Praxis to run as a Progressive Web Application compiled to WebAssembly. It provides two code paths via `#[cfg(target_arch = "wasm32")]` conditional compilation:

- **Native builds** (`cfg(not(target_arch = "wasm32"))`): A stub `BrowserAgent` with basic `process()`, `get_state()`, and `sync()` methods, plus a `storage::IndexedDBStore` stub that does nothing.
- **WASM builds** (`cfg(target_arch = "wasm32")`): A `#[wasm_bindgen]`-compatible `BrowserAgent` that accepts JSON config from JavaScript and exposes `process()`, `get_state()`, and `sync()` methods that return Rust `Result` types mapped to JavaScript promises.

The `BrowserAgentConfig` defines agent identity, capabilities (available `BrowserTool` types: Fetch, Storage, Crypto, Compute), memory limits, offline mode, sync endpoint, and `StorageProvider` (LocalStorage, IndexedDB, CloudSync).

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `BrowserAgentConfig` | Agent configuration: `agent_id`, `name`, `capabilities`, `sync_endpoint`, `storage_provider`. |
| `BrowserCapabilities` | Declared capabilities: `tools` (Vec of `BrowserTool`), `max_memory_mb`, `offline_mode`. |
| `BrowserTool` | Enum: `Fetch`, `Storage`, `Crypto`, `Compute`. |
| `StorageProvider` | Enum: `LocalStorage`, `IndexedDB`, `CloudSync`. |
| `BrowserAgent` | Agent runtime — two implementations (native stub and WASM). Methods: `new()`, `process()`, `get_state()`, `sync()`. |
| `IndexedDBStore` | Stub for IndexedDB persistence (native only): `new()`, `save()`, `load()`. |

### Platform Split

| Feature | Native (stub) | WASM |
|---------|---------------|------|
| Constructor | `new(config: BrowserAgentConfig)` | `new(config_json: &str) -> Result<BrowserAgent, JsValue>` |
| Process | `process(&mut self, input: &str) -> String` | `process(&mut self, input: &str) -> Result<String, JsValue>` |
| State | `get_state() -> String` | `get_state() -> String` |
| Sync | `sync() -> Result<(), anyhow::Error>` | `sync() -> Result<(), JsValue>` |

## Public API

```rust
// Configuration
pub struct BrowserAgentConfig {
    pub agent_id: String,
    pub name: String,
    pub capabilities: BrowserCapabilities,
    pub sync_endpoint: Option<String>,
    pub storage_provider: StorageProvider,
}

pub struct BrowserCapabilities {
    pub tools: Vec<BrowserTool>,
    pub max_memory_mb: usize,
    pub offline_mode: bool,
}

pub enum BrowserTool { Fetch, Storage, Crypto, Compute }
pub enum StorageProvider { LocalStorage, IndexedDB, CloudSync }

// Native stub
#[cfg(not(target_arch = "wasm32"))]
pub struct BrowserAgent {
    pub agent_id: String,
    pub config: BrowserAgentConfig,
    pub state: serde_json::Value,
}
impl BrowserAgent {
    pub fn new(config: BrowserAgentConfig) -> Self;
    pub fn process(&mut self, input: &str) -> String;
    pub fn get_state(&self) -> String;
    pub fn sync(&mut self) -> Result<(), anyhow::Error>;
}

// IndexedDB storage stub (native)
pub mod storage {
    pub struct IndexedDBStore;
    impl IndexedDBStore {
        pub fn new(db_name: &str, store_name: &str) -> Self;
        pub async fn save(&self, _key: &str, _value: &str) -> Result<(), anyhow::Error>;
        pub async fn load(&self, _key: &str) -> Result<Option<String>, anyhow::Error>;
    }
}

// WASM build
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl BrowserAgent {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<BrowserAgent, JsValue>;
    pub fn process(&mut self, input: &str) -> Result<String, JsValue>;
    pub fn get_state(&self) -> String;
    pub fn sync(&mut self) -> Result<(), JsValue>;
}
```

## Configuration

No `praxis.toml` section. Config is passed as JSON to the WASM constructor.

### Example (JavaScript / WASM)

```javascript
import init, { BrowserAgent } from './praxis_pwa.js';

await init();
const config = {
  agent_id: "pwa-001",
  name: "Praxis PWA",
  capabilities: {
    tools: ["Fetch", "Storage"],
    max_memory_mb: 64,
    offline_mode: true,
  },
  storage_provider: "IndexedDB",
};

const agent = new BrowserAgent(JSON.stringify(config));
const result = agent.process("hello");
console.log(result); // "Processed: hello"
```

## Dependencies

- `serde` / `serde_json` — serialization
- `wasm_bindgen` — WASM bindings (optional, wasm32 only)
- `anyhow` — error handling (native only)

## Source

`src/browser_pwa.rs`