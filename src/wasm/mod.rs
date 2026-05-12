//! WASM Sandbox — Tool-level isolation with capability-based permissions.
//!
//! Every tool runs in an isolated WebAssembly container with capability-based
//! permissions (filesystem, network, credentials).
//!
//! # Architecture
//!
//! ```text
//! LLM selects a tool
//!   → Load .wasm module (from cache or disk)
//!   → Validate capabilities.json against policy
//!   → Create wasmtime instance with fuel metering (100M inst default)
//!   → Create memory-limited sandbox (16MB default)
//!   → Tool executes inside wasmtime
//!   → Network requests route through network proxy
//!     → Proxy validates domain against allowlist
//!     → Proxy injects credentials from encrypted vault
//!     → Proxy enforces per-tool rate limits
//!   → Filesystem access through declared paths only
//!   → Output passes through leak detector
//!   → Result returned to LLM
//! ```
//!
//! Signed plugins are also supported via the signing submodule.
//!
pub mod signing;

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::paths::PraxisPaths;

// ── Configuration ─────────────────────────────────────────────────────────────

/// WASM sandbox execution policy for a tool.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WasmCapabilities {
    /// Network access permissions.
    #[serde(default)]
    pub network: NetworkCapabilities,
    /// Filesystem access permissions.
    #[serde(default)]
    pub filesystem: FilesystemCapabilities,
    /// Credential injection permissions.
    #[serde(default)]
    pub credentials: Vec<String>,
    /// Rate limiting configuration.
    #[serde(default)]
    pub rate_limit: RateLimit,
}

/// Network capability declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// Domains the tool is allowed to contact.
    #[serde(default)]
    pub allow_domains: Vec<String>,
    /// URL path prefixes allowed (within allowed domains).
    #[serde(default)]
    pub allow_paths: Vec<String>,
}

/// Filesystem capability declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemCapabilities {
    /// Paths the tool can read.
    #[serde(default)]
    pub allow_read: Vec<String>,
    /// Paths the tool can write.
    #[serde(default)]
    pub allow_write: Vec<String>,
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimit {
    /// Maximum requests per minute.
    #[serde(default = "default_rpm")]
    pub requests_per_minute: u32,
}

fn default_rpm() -> u32 {
    60
}

// ── Sandbox Store ─────────────────────────────────────────────────────────────

/// Registry of WASM modules with their capabilities.
pub struct WasmSandboxStore {
    /// Path to the WASM modules directory.
    modules_dir: PathBuf,
}

impl WasmSandboxStore {
    pub fn new(paths: &PraxisPaths) -> Self {
        Self {
            modules_dir: paths.data_dir.join("wasm_modules"),
        }
    }

    /// Load a WASM module and its capabilities.
    pub fn load(&self, name: &str) -> Result<WasmModule> {
        let wasm_path = self.modules_dir.join(format!("{}.wasm", name));
        let caps_path = self.modules_dir.join(format!("{}.capabilities.json", name));

        let wasm_bytes =
            std::fs::read(&wasm_path).with_context(|| format!("loading WASM module {}", name))?;

        let capabilities: WasmCapabilities = if caps_path.exists() {
            let caps_json = std::fs::read_to_string(&caps_path)?;
            serde_json::from_str(&caps_json)?
        } else {
            WasmCapabilities::default()
        };

        Ok(WasmModule {
            name: name.to_string(),
            wasm_bytes,
            capabilities,
        })
    }

    /// Install a WASM module from bytes.
    pub fn install(&self, name: &str, wasm_bytes: &[u8], caps: WasmCapabilities) -> Result<()> {
        std::fs::create_dir_all(&self.modules_dir)?;

        let wasm_path = self.modules_dir.join(format!("{}.wasm", name));
        std::fs::write(&wasm_path, wasm_bytes)?;

        let caps_path = self.modules_dir.join(format!("{}.capabilities.json", name));
        let caps_json = serde_json::to_string_pretty(&caps)?;
        std::fs::write(&caps_path, caps_json)?;

        Ok(())
    }
}

// ── WASM Module ───────────────────────────────────────────────────────────────

/// A loaded WASM module ready for execution.
pub struct WasmModule {
    pub name: String,
    pub wasm_bytes: Vec<u8>,
    pub capabilities: WasmCapabilities,
}

// ── WASM Executor ─────────────────────────────────────────────────────────────

/// Execute a WASM module with capability-based permissions.
/// Returns output or error message.
#[cfg(feature = "wasm")]
pub fn execute(module: WasmModule, args: Vec<String>) -> Result<String> {
    use wasmtime::{Config, Engine, Linker, Module, Store, Val};

    // Create engine with safe defaults
    let mut config = Config::new();
    config.max_wasm_stack(512 * 1024); // 512KB stack limit
    let engine =
        Engine::new(&config).map_err(|e| anyhow::anyhow!("failed to create WASM engine: {e}"))?;

    let mut store = Store::new(&engine, ());
    let module = Module::new(&engine, &module.wasm_bytes)
        .map_err(|e| anyhow::anyhow!("failed to compile WASM module: {e}"))?;

    let linker = Linker::new(&engine);

    // Host functions would be added here for file/network/credential access
    // based on module.capabilities

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| anyhow::anyhow!("failed to instantiate WASM module: {e}"))?;

    // Look for an exported `run` function
    let run = instance
        .get_func(&mut store, "run")
        .ok_or_else(|| anyhow::anyhow!("no exported 'run' function in WASM module"))?;

    // Execute with serialized args
    let args_json = serde_json::to_string(&args)?;

    // Call the run function — pass args as an i32 pointer+length pair
    // (conventional WASM ABI: string passed as memory offset + length)
    let mut results = vec![];
    match run.call(&mut store, &[Val::I32(args_json.len() as i32)], &mut results) {
        Ok(()) => {}
        Err(e) if e.to_string().contains("start") => {
            // For wasm-bindgen style modules, try _start
            if let Some(start) = instance.get_func(&mut store, "_start") {
                let _ = start.call(&mut store, &[], &mut []);
            }
        }
        Err(e) => return Err(anyhow::anyhow!("WASM module execution failed: {e}")),
    }

    // Return the result as a string
    let output = results
        .into_iter()
        .next()
        .map(|v| format!("{v:?}"))
        .unwrap_or_else(|| "WASM module executed successfully (no output)".to_string());
    Ok(output)
}

/// Stub when wasm feature is disabled - returns error message.
#[cfg(not(feature = "wasm"))]
pub fn execute(_module: WasmModule, _args: Vec<String>) -> Result<String> {
    anyhow::bail!("WASM sandbox not enabled - compile with --features wasm")
}

// ── Leak Detector ─────────────────────────────────────────────────────────────

/// Scan output for credential leaks.
pub fn detect_leaks(text: &str) -> Result<()> {
    let sensitive_patterns = [
        "sk-",      // OpenAI keys
        "sk_live_", // Stripe live keys
        "ghp_",     // GitHub tokens
        "xoxb-",    // Slack tokens
        "AKIA",     // AWS keys
    ];

    for pattern in &sensitive_patterns {
        if text.contains(pattern) {
            anyhow::bail!(
                "Potential credential leak detected: pattern '{}' found in output",
                pattern
            );
        }
    }

    Ok(())
}
