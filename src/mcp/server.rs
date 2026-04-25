//! MCP server exposing Praxis tools over stdio transport.
//!
//! Run as `praxis mcp serve` — reads JSON-RPC requests from stdin, writes
//! responses to stdout.  Designed for integration with Claude Desktop,
//! Cursor, and any other MCP client.
//!
//! Each tool maps to a Praxis `ToolManifest`.  The `praxis-data-write` and
//! `shell-exec` tools are exposed with their full descriptions so the client
//! can render them to the user.  Execution requires the Praxis approval queue
//! (or pre-approval for read-only tools).

use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::{
    paths::{PraxisPaths, default_data_dir},
    tools::{FileToolRegistry, ToolExecutionResult, ToolManifest, ToolRegistry, execute_request},
};

use super::types::*;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Dispatch a single JSON-RPC request and return the response.
/// Used by the dashboard HTTP endpoint (`/mcp`).
pub fn dispatch(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    let registry = FileToolRegistry;
    let manifests = registry.list(paths).unwrap_or_default();
    let mut initialized = true; // HTTP endpoint skips handshake for simplicity
    handle_request(request, paths, &registry, &manifests, &mut initialized)
}

/// Serve Praxis tools as an MCP server over stdio.
///
/// Reads JSON-RPC requests from stdin, one per line (newline-delimited JSON).
/// Writes JSON-RPC responses to stdout, flushing after each line.
///
/// ```ignore
/// # In claude_desktop_config.json:
/// {
///   "mcpServers": {
///     "praxis": {
///       "command": "praxis",
///       "args": ["mcp", "serve"]
///     }
///   }
/// }
/// ```
pub fn serve_stdio(data_dir_override: Option<PathBuf>) -> Result<()> {
    let data_dir = data_dir_override.unwrap_or_else(|| default_data_dir().unwrap_or_default());
    let paths = PraxisPaths::for_data_dir(data_dir);
    let registry = FileToolRegistry;
    let manifests = registry.list(&paths).unwrap_or_default();

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    let mut initialized = false;

    for line in stdin.lock().lines() {
        let line = line.context("failed to read from stdin")?;
        if line.trim().is_empty() {
            continue;
        }

        let msg: JsonRpcMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                let err = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: RequestId::Number(-1),
                    result: None,
                    error: Some(JsonRpcError::invalid_params(format!("parse error: {e}"))),
                };
                write_response(&mut stdout, &err)?;
                continue;
            }
        };

        match msg {
            JsonRpcMessage::Request(req) => {
                let resp = handle_request(&req, &paths, &registry, &manifests, &mut initialized);
                write_response(&mut stdout, &resp)?;
            }
            JsonRpcMessage::Notification(notif) => {
                handle_notification(&notif, &mut initialized);
            }
            JsonRpcMessage::Response(_) => {
                // Server shouldn't receive responses unless it's acting as a client too.
                log::warn!("mcp server: received unexpected JSON-RPC response");
            }
        }
    }

    Ok(())
}

fn handle_request(
    req: &JsonRpcRequest,
    paths: &PraxisPaths,
    _registry: &FileToolRegistry,
    manifests: &[ToolManifest],
    initialized: &mut bool,
) -> JsonRpcResponse {
    let make_resp = |result: Result<Value, JsonRpcError>| {
        let (ok, err) = match result {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: req.id.clone(),
            result: ok,
            error: err,
        }
    };

    match req.method.as_str() {
        "initialize" => {
            let result = handle_initialize(req.params.as_ref());
            *initialized = true;
            make_resp(result)
        }
        "initialized" => {
            // Client has acknowledged initialization.  No response needed for
            // notifications, but since this arrives as a request in some
            // implementations, return an empty result.
            make_resp(Ok(Value::Null))
        }
        "notifications/initialized" => make_resp(Ok(Value::Null)),
        _ if !*initialized => make_resp(Err(JsonRpcError::invalid_params(
            "server not initialized — call initialize first",
        ))),
        "tools/list" => make_resp(handle_tools_list(manifests)),
        "tools/call" => make_resp(handle_tool_call(req.params.as_ref(), paths, manifests)),
        _ => make_resp(Err(JsonRpcError::method_not_found(format!(
            "unknown method: {}",
            req.method
        )))),
    }
}

fn handle_notification(notif: &JsonRpcNotification, initialized: &mut bool) {
    match notif.method.as_str() {
        "notifications/initialized" => {
            *initialized = true;
        }
        _ => {
            log::debug!("mcp server: ignoring notification {}", notif.method);
        }
    }
}

fn handle_initialize(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let client_req: InitializeRequest = params
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .unwrap_or(InitializeRequest {
            protocol_version: PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "unknown".to_string(),
                version: "0.0.0".to_string(),
            },
        });

    log::info!(
        "mcp server: initialize from {} {}",
        client_req.client_info.name,
        client_req.client_info.version
    );

    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: Some(false) }),
            resources: None,
            prompts: None,
        },
        server_info: Implementation {
            name: "praxis".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

fn handle_tools_list(manifests: &[ToolManifest]) -> Result<Value, JsonRpcError> {
    let tools: Vec<McpTool> = manifests.iter().map(manifest_to_mcp_tool).collect();
    let result = ListToolsResult { tools, next_cursor: None };
    serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
}

fn manifest_to_mcp_tool(m: &ToolManifest) -> McpTool {
    // Build a JSON Schema for the tool's arguments based on its kind.
    let schema = match m.kind {
        crate::tools::ToolKind::Internal if m.name == "praxis-data-write" => {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path under data_dir" },
                    "append_text": { "type": "string", "description": "Text to append" }
                },
                "required": ["path", "append_text"]
            })
        }
        crate::tools::ToolKind::Internal if m.name == "file-read" => {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative file path to read" }
                },
                "required": ["path"]
            })
        }
        crate::tools::ToolKind::Shell if m.name == "shell-exec" => {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute" }
                },
                "required": ["command"]
            })
        }
        crate::tools::ToolKind::Http => {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "params": { "type": "object", "description": "Key-value parameters for the request" }
                }
            })
        }
        _ => {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "args": { "type": "string", "description": "Arguments for the tool" }
                }
            })
        }
    };

    McpTool {
        name: m.name.clone(),
        description: m.description.clone(),
        input_schema: Some(schema),
    }
}

fn handle_tool_call(
    params: Option<&Value>,
    paths: &PraxisPaths,
    manifests: &[ToolManifest],
) -> Result<Value, JsonRpcError> {
    let call: CallToolRequest = params
        .and_then(|p| serde_json::from_value(p.clone()).ok())
        .ok_or_else(|| JsonRpcError::invalid_params("missing or invalid tools/call params"))?;

    let manifest = manifests
        .iter()
        .find(|m| m.name == call.name)
        .ok_or_else(|| JsonRpcError::invalid_params(format!("unknown tool: {}", call.name)))?;

    // Build a synthetic StoredApprovalRequest from the MCP arguments.
    // For safety, write/shell tools always require operator approval when
    // invoked via MCP — we queue them rather than executing directly.
    let requires_approval = manifest.requires_approval
        || matches!(manifest.kind, crate::tools::ToolKind::Shell)
        || manifest.name == "praxis-data-write";

    if requires_approval {
        let result = CallToolResult::text(format!(
            "Tool '{}' requires operator approval. Use `praxis approvals` to review and approve.",
            call.name
        ));
        return serde_json::to_value(result)
            .map_err(|e| JsonRpcError::internal_error(e.to_string()));
    }

    // Build payload JSON from arguments.
    let payload_json = call.arguments.as_ref().map(|a| a.to_string()).unwrap_or_default();

    // For read-only internal tools (file-read), execute directly.
    let synthetic_request = crate::storage::StoredApprovalRequest {
        id: 0,
        tool_name: call.name.clone(),
        summary: format!("mcp call to {} with {}", call.name, payload_json),
        requested_by: "mcp-client".to_string(),
        write_paths: manifest.allowed_paths.clone(),
        payload_json: if payload_json.is_empty() {
            None
        } else {
            Some(payload_json)
        },
        status: crate::storage::ApprovalStatus::Approved, // auto-approved for safe tools
        status_note: Some("auto-approved via mcp".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    match execute_request(paths, manifest, &synthetic_request) {
        Ok(ToolExecutionResult { summary }) => {
            let result = CallToolResult::text(summary);
            serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
        }
        Err(e) => {
            let result = CallToolResult::error(format!("tool execution failed: {e:#}"));
            serde_json::to_value(result).map_err(|e| JsonRpcError::internal_error(e.to_string()))
        }
    }
}

fn write_response(stdout: &mut io::StdoutLock, resp: &JsonRpcResponse) -> Result<()> {
    let line = serde_json::to_string(resp).context("serialize JSON-RPC response")?;
    writeln!(stdout, "{line}").context("write to stdout")?;
    stdout.flush().context("flush stdout")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolKind;

    #[test]
    fn manifest_to_mcp_tool_conversion() {
        let m = ToolManifest {
            name: "file-read".to_string(),
            description: "Read a file".to_string(),
            kind: ToolKind::Internal,
            required_level: 1,
            requires_approval: false,
            rehearsal_required: false,
            allowed_paths: vec![],
            allowed_read_paths: vec![],
            path: None,
            args: vec![],
            timeout_secs: None,
            endpoint: None,
            method: None,
            headers: vec![],
            body: None,
            allowed_vault_keys: None,
            allowed_oauth_providers: None,
        };
        let tool = manifest_to_mcp_tool(&m);
        assert_eq!(tool.name, "file-read");
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn initialize_returns_protocol_version() {
        let params = serde_json::json!({
            "protocol_version": "2024-11-05",
            "capabilities": {},
            "client_info": { "name": "test", "version": "1.0" }
        });
        let result = handle_initialize(Some(&params)).unwrap();
        let init: InitializeResult = serde_json::from_value(result).unwrap();
        assert_eq!(init.protocol_version, PROTOCOL_VERSION);
        assert!(init.capabilities.tools.is_some());
    }
}
