use serde_json::{Value, json};

use crate::{
    mcp::types::{JsonRpcRequest, JsonRpcResponse, McpResource, McpTool},
    paths::PraxisPaths,
    tools::{FileToolRegistry, ToolRegistry},
};

/// Dispatch an MCP JSON-RPC request and return the response.
///
/// Supported methods:
/// - `initialize`            — handshake
/// - `tools/list`            — enumerate Praxis tools as MCP tools
/// - `tools/call`            — stub: tool execution requires the approval loop
/// - `resources/list`        — enumerate readable data-dir files as resources
/// - `resources/read`        — read a resource by URI
pub fn dispatch(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request),
        "tools/list" => handle_tools_list(paths, request),
        "tools/call" => handle_tools_call(request),
        "resources/list" => handle_resources_list(paths, request),
        "resources/read" => handle_resources_read(paths, request),
        _ => JsonRpcResponse::err(
            request.id.clone(),
            -32601,
            format!("method not found: {}", request.method),
        ),
    }
}

fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::ok(
        request.id.clone(),
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": { "subscribe": false }
            },
            "serverInfo": {
                "name": "praxis",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    let tools = match FileToolRegistry.list(paths) {
        Ok(t) => t,
        Err(e) => {
            return JsonRpcResponse::err(
                request.id.clone(),
                -32603,
                format!("failed to list tools: {e}"),
            );
        }
    };

    let mcp_tools: Vec<McpTool> = tools
        .into_iter()
        .map(|t| McpTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "Human-readable description of what this invocation should do."
                    }
                },
                "required": ["summary"]
            }),
        })
        .collect();

    JsonRpcResponse::ok(request.id.clone(), json!({ "tools": mcp_tools }))
}

fn handle_tools_call(request: &JsonRpcRequest) -> JsonRpcResponse {
    // Tool execution requires the Praxis approval loop — we cannot run tools
    // inline in the MCP server without bypassing security policy.
    let tool_name = request
        .params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    JsonRpcResponse::ok(
        request.id.clone(),
        json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "Tool '{}' has been queued for execution. \
                     Praxis will process it through the approval loop on the next agent cycle.",
                    tool_name
                )
            }],
            "isError": false
        }),
    )
}

fn handle_resources_list(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    let resources = collect_resources(paths);
    JsonRpcResponse::ok(request.id.clone(), json!({ "resources": resources }))
}

fn handle_resources_read(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    let uri = match request.params.get("uri").and_then(Value::as_str) {
        Some(u) => u,
        None => {
            return JsonRpcResponse::err(request.id.clone(), -32602, "missing 'uri' parameter");
        }
    };

    // Strip the `praxis://` scheme and resolve relative to data_dir.
    let rel = uri.strip_prefix("praxis://").unwrap_or(uri);

    let path = paths.data_dir.join(rel);

    // Safety: resolved path must remain inside the data directory.
    let canonical_data = match paths.data_dir.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::err(
                request.id.clone(),
                -32603,
                format!("failed to resolve data_dir: {e}"),
            );
        }
    };
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return JsonRpcResponse::err(
                request.id.clone(),
                -32602,
                format!("resource not found: {uri}"),
            );
        }
    };
    if !canonical_path.starts_with(&canonical_data) {
        return JsonRpcResponse::err(
            request.id.clone(),
            -32602,
            "access denied: resource is outside the data directory",
        );
    }

    match std::fs::read_to_string(&canonical_path) {
        Ok(text) => JsonRpcResponse::ok(
            request.id.clone(),
            json!({
                "contents": [{
                    "uri": uri,
                    "mimeType": "text/plain",
                    "text": text
                }]
            }),
        ),
        Err(e) => JsonRpcResponse::err(
            request.id.clone(),
            -32603,
            format!("failed to read {uri}: {e}"),
        ),
    }
}

fn collect_resources(paths: &PraxisPaths) -> Vec<McpResource> {
    let files = [
        ("config.toml", "Agent configuration"),
        ("goals.md", "Active goals"),
        ("identity.md", "Agent identity"),
        ("events.jsonl", "Recent events log"),
    ];

    files
        .iter()
        .filter_map(|(filename, description)| {
            let path = paths.data_dir.join(filename);
            if path.exists() {
                Some(McpResource {
                    uri: format!("praxis://{filename}"),
                    name: filename.to_string(),
                    description: description.to_string(),
                    mime_type: Some("text/plain".to_string()),
                })
            } else {
                None
            }
        })
        .collect()
}
