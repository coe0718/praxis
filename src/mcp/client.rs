//! MCP client — connects to external MCP servers and surfaces their tools.
//!
//! Spawns an MCP server as a subprocess, communicates over stdio, and
//! registers discovered tools in the Praxis tool registry.

use std::{
    io::{BufRead, Write},
    process::{Child, Command, Stdio},
};

use anyhow::{Context, Result};

use super::types::*;

pub struct McpClient {
    child: Child,
}

impl McpClient {
    /// Spawn an MCP server command and establish a stdio connection.
    ///
    /// ```no_run
    /// let client = McpClient::spawn("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])?;
    /// ```
    pub fn spawn(cmd: &str, args: &[&str]) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("failed to spawn MCP server: {cmd}"))?;

        // Perform initialization handshake.
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(0),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocol_version": "2024-11-05",
                "capabilities": {},
                "client_info": { "name": "praxis", "version": env!("CARGO_PKG_VERSION") }
            })),
        };

        let stdin = child.stdin.as_mut().context("no stdin on child process")?;
        let stdout = child.stdout.as_mut().context("no stdout on child process")?;

        Self::write_request(stdin, &init_req)?;
        let _resp: JsonRpcResponse = Self::read_response(stdout)?;

        // Send initialized notification.
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/initialized".to_string(),
            params: None,
        };
        Self::write_notification(stdin, &notif)?;

        Ok(Self { child })
    }

    /// List tools exposed by the connected MCP server.
    pub fn list_tools(&mut self) -> Result<Vec<McpTool>> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(1),
            method: "tools/list".to_string(),
            params: None,
        };

        let stdin = self.child.stdin.as_mut().context("no stdin")?;
        let stdout = self.child.stdout.as_mut().context("no stdout")?;

        Self::write_request(stdin, &req)?;
        let resp = Self::read_response(stdout)?;

        let result = resp.result.context("tools/list returned no result")?;
        let list: ListToolsResult =
            serde_json::from_value(result).context("invalid tools/list response")?;
        Ok(list.tools)
    }

    /// Call a tool on the connected MCP server.
    pub fn call_tool(&mut self, name: &str, arguments: Option<serde_json::Value>) -> Result<CallToolResult> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: RequestId::Number(2),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": name,
                "arguments": arguments.unwrap_or(serde_json::Value::Null)
            })),
        };

        let stdin = self.child.stdin.as_mut().context("no stdin")?;
        let stdout = self.child.stdout.as_mut().context("no stdout")?;

        Self::write_request(stdin, &req)?;
        let resp = Self::read_response(stdout)?;

        let result = resp.result.context("tools/call returned no result")?;
        let call_result: CallToolResult =
            serde_json::from_value(result).context("invalid tools/call response")?;
        Ok(call_result)
    }

    fn write_request(stdin: &mut std::process::ChildStdin, req: &JsonRpcRequest) -> Result<()> {
        let line = serde_json::to_string(req).context("serialize JSON-RPC request")?;
        writeln!(stdin, "{line}").context("write to MCP stdin")?;
        stdin.flush().context("flush MCP stdin")
    }

    fn write_notification(stdin: &mut std::process::ChildStdin, notif: &JsonRpcNotification) -> Result<()> {
        let line = serde_json::to_string(notif).context("serialize JSON-RPC notification")?;
        writeln!(stdin, "{line}").context("write notification to MCP stdin")?;
        stdin.flush().context("flush MCP stdin")
    }

    fn read_response(stdout: &mut std::process::ChildStdout) -> Result<JsonRpcResponse> {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.context("read from MCP stdout")?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(JsonRpcMessage::Response(resp)) = serde_json::from_str(&line) {
                return Ok(resp);
            }
            // Skip notifications / other messages until we get a response.
        }
        anyhow::bail!("MCP server closed stdout before sending a response")
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
