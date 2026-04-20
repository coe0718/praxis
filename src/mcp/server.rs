use serde_json::{Value, json};

use crate::{
    mcp::types::{JsonRpcRequest, JsonRpcResponse, McpResource, McpTool},
    paths::PraxisPaths,
    storage::{
        ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionStore, SqliteSessionStore,
    },
    tools::{FileToolRegistry, ToolKind, ToolManifest, ToolRegistry},
};

/// Dispatch an MCP JSON-RPC request and return the response.
///
/// Supported methods:
/// - `initialize`            — handshake
/// - `tools/list`            — enumerate Praxis tools as MCP tools
/// - `tools/call`            — queue a tool approval request in SQLite
/// - `resources/list`        — enumerate readable data-dir files as resources
/// - `resources/read`        — read a resource by URI
pub fn dispatch(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => handle_initialize(request),
        "tools/list" => handle_tools_list(paths, request),
        "tools/call" => handle_tools_call(paths, request),
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
            input_schema: tool_input_schema(&t),
        })
        .collect();

    JsonRpcResponse::ok(request.id.clone(), json!({ "tools": mcp_tools }))
}

/// Queue a tool approval request in SQLite so the next agent cycle can pick it up.
///
/// Expected params:
/// - `name`              — tool name (required)
/// - `arguments.summary` — human-readable intent (required)
/// - `arguments.params`  — key/value map of tool-specific parameters (optional)
/// - `arguments.write_paths` — array of paths the tool may write to (optional)
fn handle_tools_call(paths: &PraxisPaths, request: &JsonRpcRequest) -> JsonRpcResponse {
    let tool_name = match request.params.get("name").and_then(Value::as_str) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => {
            return JsonRpcResponse::err(
                request.id.clone(),
                -32602,
                "tools/call requires a 'name' parameter",
            );
        }
    };

    let args = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or(json!({}));

    let summary = args
        .get("summary")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("MCP-requested call to {tool_name}"));

    let write_paths: Vec<String> = args
        .get("write_paths")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    // Serialize the params sub-object (or the whole arguments) as the payload.
    let payload_json = args
        .get("params")
        .map(|p| serde_json::to_string(p).unwrap_or_default())
        .or_else(|| {
            if args.as_object().is_none_or(|o| o.is_empty()) {
                None
            } else {
                serde_json::to_string(&args).ok()
            }
        });

    let store = SqliteSessionStore::new(paths.database_file.clone());
    if let Err(e) = store.initialize() {
        return JsonRpcResponse::err(
            request.id.clone(),
            -32603,
            format!("failed to initialise store: {e}"),
        );
    }

    match store.queue_approval(&NewApprovalRequest {
        tool_name: tool_name.clone(),
        summary: summary.clone(),
        requested_by: "mcp".to_string(),
        write_paths,
        payload_json,
        status: ApprovalStatus::Pending,
    }) {
        Ok(stored) => JsonRpcResponse::ok(
            request.id.clone(),
            json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Tool '{}' queued as approval request #{} (status: pending). \
                         An operator must approve it before the next agent cycle executes it.",
                        tool_name, stored.id
                    )
                }],
                "isError": false
            }),
        ),
        Err(e) => JsonRpcResponse::err(
            request.id.clone(),
            -32603,
            format!("failed to queue tool approval: {e}"),
        ),
    }
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
    let candidates: &[(&std::path::Path, &str)] = &[
        (paths.config_file.as_path(), "Agent configuration"),
        (paths.goals_file.as_path(), "Active goals"),
        (paths.identity_file.as_path(), "Agent working identity"),
        (paths.soul_file.as_path(), "Agent core identity (immutable)"),
        (
            paths.agents_file.as_path(),
            "Agent conventions and quality gates",
        ),
        (
            paths.capabilities_file.as_path(),
            "Installed tools and skills index",
        ),
        (paths.events_file.as_path(), "Recent event log"),
        (paths.journal_file.as_path(), "Agent journal"),
        (paths.patterns_file.as_path(), "Observed patterns"),
    ];

    candidates
        .iter()
        .filter_map(|(path, description)| {
            if !path.exists() {
                return None;
            }
            let filename = path.file_name()?.to_str()?;
            Some(McpResource {
                uri: format!("praxis://{filename}"),
                name: filename.to_string(),
                description: description.to_string(),
                mime_type: Some("text/plain".to_string()),
            })
        })
        .collect()
}

/// Build an MCP input schema tailored to the tool's kind and known parameters.
fn tool_input_schema(manifest: &ToolManifest) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "summary".to_string(),
        json!({
            "type": "string",
            "description": "Human-readable description of what this invocation should do."
        }),
    );

    match manifest.kind {
        ToolKind::Shell => {
            properties.insert(
                "params".to_string(),
                json!({
                    "type": "object",
                    "description": "Key/value parameters passed to the tool (e.g. {\"args\": \"--flag value\"}).",
                    "properties": {
                        "args": { "type": "string", "description": "Extra command-line arguments." }
                    }
                }),
            );
        }
        ToolKind::Http => {
            properties.insert(
                "params".to_string(),
                json!({
                    "type": "object",
                    "description": "Key/value parameters substituted into the endpoint URL and body templates.",
                    "additionalProperties": { "type": "string" }
                }),
            );
        }
        ToolKind::Internal => {}
    }

    properties.insert(
        "write_paths".to_string(),
        json!({
            "type": "array",
            "items": { "type": "string" },
            "description": "Paths (relative to data_dir) the tool is permitted to write."
        }),
    );

    json!({
        "type": "object",
        "properties": properties,
        "required": ["summary"]
    })
}
