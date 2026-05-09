//! Browser-Only PWA Mode — Agent runs entirely in browser tab.
//!
//! Compile Praxis agent to WebAssembly for client-side execution.
//! State synced via cloud storage, identity via WebCrypto.

use serde::{Deserialize, Serialize};

/// Browser-compatible agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAgentConfig {
    pub agent_id: String,
    pub name: String,
    pub capabilities: BrowserCapabilities,
    pub sync_endpoint: Option<String>,
    pub storage_provider: StorageProvider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCapabilities {
    pub tools: Vec<BrowserTool>,
    pub max_memory_mb: usize,
    pub offline_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserTool {
    Fetch,
    Storage,
    Crypto,
    Compute,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum StorageProvider {
    LocalStorage,
    IndexedDB,
    CloudSync,
}

/// WASM agent state manager (stub for native builds).
#[cfg(not(target_arch = "wasm32"))]
pub struct BrowserAgent {
    pub agent_id: String,
    pub config: BrowserAgentConfig,
    pub state: serde_json::Value,
}

#[cfg(not(target_arch = "wasm32"))]
impl BrowserAgent {
    pub fn new(config: BrowserAgentConfig) -> Self {
        Self {
            agent_id: config.agent_id.clone(),
            config,
            state: serde_json::json!({}),
        }
    }

    pub fn process(&mut self, input: &str) -> String {
        format!("Processed: {}", input)
    }

    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap_or_default()
    }

    pub fn sync(&mut self) -> Result<(), anyhow::Error> {
        Ok(())
    }
}

/// WASM agent for actual browser builds.
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct BrowserAgent {
    agent_id: String,
    config: BrowserAgentConfig,
    state: serde_json::Value,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl BrowserAgent {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<BrowserAgent, JsValue> {
        let config: BrowserAgentConfig =
            serde_json::from_str(config_json).map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(BrowserAgent {
            agent_id: config.agent_id.clone(),
            config,
            state: serde_json::json!({}),
        })
    }

    #[wasm_bindgen]
    pub fn process(&mut self, input: &str) -> Result<String, JsValue> {
        Ok(format!("Processed: {}", input))
    }

    #[wasm_bindgen]
    pub fn get_state(&self) -> String {
        serde_json::to_string(&self.state).unwrap_or_default()
    }

    #[wasm_bindgen]
    pub fn sync(&mut self) -> Result<(), JsValue> {
        Ok(())
    }
}

/// IndexedDB state persistence stub.
#[cfg(not(target_arch = "wasm32"))]
pub mod storage {
    // IndexedDBStore is self-contained, no need for super::*

    pub struct IndexedDBStore {
        pub db_name: String,
        pub store_name: String,
    }

    impl IndexedDBStore {
        pub fn new(db_name: &str, store_name: &str) -> Self {
            Self {
                db_name: db_name.to_string(),
                store_name: store_name.to_string(),
            }
        }

        pub async fn save(&self, _key: &str, _value: &str) -> Result<(), anyhow::Error> {
            Ok(())
        }

        pub async fn load(&self, _key: &str) -> Result<Option<String>, anyhow::Error> {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_agent_config() {
        let config = BrowserAgentConfig {
            agent_id: "browser_001".to_string(),
            name: "PWA Agent".to_string(),
            capabilities: BrowserCapabilities {
                tools: vec![BrowserTool::Fetch, BrowserTool::Storage],
                max_memory_mb: 64,
                offline_mode: true,
            },
            sync_endpoint: None,
            storage_provider: StorageProvider::LocalStorage,
        };

        assert_eq!(config.agent_id, "browser_001");
    }

    #[test]
    fn test_browser_agent() {
        let config = BrowserAgentConfig {
            agent_id: "test".to_string(),
            name: "Test".to_string(),
            capabilities: BrowserCapabilities {
                tools: vec![],
                max_memory_mb: 32,
                offline_mode: false,
            },
            sync_endpoint: None,
            storage_provider: StorageProvider::CloudSync,
        };

        let mut agent = BrowserAgent::new(config);
        let result = agent.process("hello");
        assert!(result.contains("hello"));
    }
}
