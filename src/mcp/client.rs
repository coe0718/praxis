use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse, McpServerConfig, McpTool};

/// MCP client that speaks to an external MCP server over HTTP.
pub struct McpClient {
    client: Client,
    config: McpServerConfig,
    next_id: std::sync::atomic::AtomicU64,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .context("failed to build HTTP client")?,
            config,
            next_id: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Perform the MCP `initialize` handshake.
    pub fn initialize(&self) -> Result<Value> {
        let resp = self.call(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "praxis",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )?;
        Ok(resp)
    }

    /// List all tools exposed by the remote server.
    pub fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.call("tools/list", json!({}))?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let parsed: Vec<McpTool> = serde_json::from_value(Value::Array(tools))
            .context("failed to parse MCP tools list")?;
        Ok(parsed)
    }

    /// Call a remote tool by name with the given arguments.
    pub fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        self.call(
            "tools/call",
            json!({ "name": name, "arguments": arguments }),
        )
    }

    /// List resources exposed by the remote server.
    pub fn list_resources(&self) -> Result<Vec<Value>> {
        let result = self.call("resources/list", json!({}))?;
        Ok(result
            .get("resources")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// Read a resource by URI from the remote server.
    pub fn read_resource(&self, uri: &str) -> Result<Value> {
        self.call("resources/read", json!({ "uri": uri }))
    }

    fn next_id(&self) -> u64 {
        self.next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(id.into())),
            method: method.to_string(),
            params,
        };

        let url = format!("{}/mcp", self.config.url.trim_end_matches('/'));

        let mut builder = self.client.post(&url).json(&request);

        if let Some(token) = &self.config.token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }

        let response = builder
            .send()
            .with_context(|| format!("failed to reach MCP server {}", self.config.name))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("MCP server {} returned {status}: {body}", self.config.name);
        }

        let rpc_response: JsonRpcResponse = response.json().with_context(|| {
            format!(
                "failed to parse JSON-RPC response from {}",
                self.config.name
            )
        })?;

        if let Some(error) = rpc_response.error {
            bail!(
                "MCP error {}: {} (code {})",
                self.config.name,
                error.message,
                error.code
            );
        }

        Ok(rpc_response.result.unwrap_or(Value::Null))
    }
}
