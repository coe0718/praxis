//! ACP (Agent Communication Protocol) server for IDE integration.
//!
//! Implements JSON-RPC 2.0 over stdio, compatible with Claude Code, Cursor,
//! Windsurf, and other ACP-capable editors.

use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::paths::{PraxisPaths, default_data_dir};

// ── JSON-RPC types ──

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// Standard JSON-RPC error codes
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

// ── ACP method handlers ──

/// Run the ACP server, reading JSON-RPC from stdin and writing responses to stdout.
pub fn run_acp_server(data_dir_override: Option<PathBuf>) -> Result<()> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut line = String::new();
    let mut reader = stdin.lock();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                log::error!("ACP: failed to read from stdin: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => handle_request(&paths, &req),
            Err(e) => error_response(None, PARSE_ERROR, &format!("Parse error: {e}")),
        };

        let mut output = serde_json::to_string(&response)?;
        output.push('\n');
        stdout.write_all(output.as_bytes())?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    if req.jsonrpc != "2.0" {
        return error_response(req.id.clone(), INVALID_REQUEST, "Expected jsonrpc 2.0");
    }

    match req.method.as_str() {
        "initialize" => handle_initialize(req),
        "initialized" => ok_response(req.id.clone(), Value::Null),
        "shutdown" => ok_response(req.id.clone(), Value::Null),
        "exit" => ok_response(req.id.clone(), Value::Null),
        "tools/list" => handle_tools_list(paths, req),
        "tools/call" => handle_tools_call(paths, req),
        "chat" => handle_chat(paths, req),
        "context/get" => handle_context_get(paths, req),
        "status" => handle_status(paths, req),
        _ => error_response(
            req.id.clone(),
            METHOD_NOT_FOUND,
            &format!("Unknown method: {}", req.method),
        ),
    }
}

fn handle_initialize(req: &JsonRpcRequest) -> JsonRpcResponse {
    ok_response(
        req.id.clone(),
        json!({
            "name": "praxis",
            "version": env!("CARGO_PKG_VERSION"),
            "protocolVersion": "0.1.0",
            "capabilities": {
                "tools": true,
                "chat": true,
                "context": true,
                "streaming": false,
            }
        }),
    )
}

fn handle_tools_list(paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    let mut tools = Vec::new();

    // Scan tools directory for manifest files
    if paths.tools_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&paths.tools_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".toml") {
                        let tool_name = name.trim_end_matches(".toml");
                        tools.push(json!({
                            "name": tool_name,
                            "description": format!("Tool: {tool_name}"),
                        }));
                    }
                }
            }
        }
    }

    // Add built-in tools
    for builtin in &["clarify", "todo", "memory", "cron", "image", "vision", "voice"] {
        tools.push(json!({
            "name": builtin,
            "description": format!("Built-in {builtin} tool"),
        }));
    }

    ok_response(req.id.clone(), json!({ "tools": tools }))
}

fn handle_tools_call(_paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    let params = &req.params;
    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name,
        None => return error_response(req.id.clone(), INVALID_PARAMS, "Missing 'name' parameter"),
    };

    let tool_params = params.get("arguments").cloned().unwrap_or(json!({}));

    // Dispatch to the appropriate tool handler
    match tool_name {
        "clarify" => {
            let question = tool_params.get("question").and_then(|v| v.as_str()).unwrap_or("");
            ok_response(req.id.clone(), json!({ "result": format!("Clarify: {question}") }))
        }
        "todo" => {
            let action = tool_params.get("action").and_then(|v| v.as_str()).unwrap_or("list");
            ok_response(req.id.clone(), json!({ "result": format!("Todo action: {action}") }))
        }
        "memory" => {
            let action = tool_params.get("action").and_then(|v| v.as_str()).unwrap_or("list");
            ok_response(req.id.clone(), json!({ "result": format!("Memory action: {action}") }))
        }
        _ => {
            // For tools with TOML manifests, return a placeholder
            ok_response(
                req.id.clone(),
                json!({ "result": format!("Tool '{tool_name}' executed (params: {tool_params})") }),
            )
        }
    }
}

fn handle_chat(paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    let message = req.params.get("message").and_then(|v| v.as_str()).unwrap_or("");

    if message.is_empty() {
        return error_response(req.id.clone(), INVALID_PARAMS, "Missing 'message' parameter");
    }

    // Delegate to the ask handler
    let ask_args = crate::cli::AskArgs {
        files: Vec::new(),
        attachment_policy: "reject".to_string(),
        tools: false,
        prompt: message.split_whitespace().map(String::from).collect(),
    };

    match crate::cli::core::handle_ask(Some(paths.data_dir.clone()), ask_args) {
        Ok(response) => ok_response(req.id.clone(), json!({ "response": response })),
        Err(e) => error_response(req.id.clone(), INTERNAL_ERROR, &format!("Chat error: {e:#}")),
    }
}

fn handle_context_get(paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    let mut context_files = Vec::new();

    let identity_files = [
        ("soul", &paths.soul_file),
        ("identity", &paths.identity_file),
        ("goals", &paths.goals_file),
        ("agents", &paths.agents_file),
        ("capabilities", &paths.capabilities_file),
    ];

    for (name, path) in &identity_files {
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                context_files.push(json!({
                    "name": name,
                    "path": path.display().to_string(),
                    "content": content,
                    "size": content.len(),
                }));
            }
        }
    }

    ok_response(req.id.clone(), json!({ "files": context_files }))
}

fn handle_status(paths: &PraxisPaths, req: &JsonRpcRequest) -> JsonRpcResponse {
    use crate::storage::SqliteSessionStore;

    let store = SqliteSessionStore::new(paths.database_file.clone());
    let hot = store.count_hot_memories().unwrap_or(0);
    let cold = store.count_cold_memories().unwrap_or(0);
    let pending = store.count_pending_approvals().unwrap_or(0);

    ok_response(
        req.id.clone(),
        json!({
            "status": "running",
            "version": env!("CARGO_PKG_VERSION"),
            "memory": { "hot": hot, "cold": cold },
            "pending_approvals": pending,
        }),
    )
}

// ── Response helpers ──

fn ok_response(id: Option<Value>, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_paths() -> PraxisPaths {
        PraxisPaths::for_data_dir(PathBuf::from("/tmp/test-praxis-acp"))
    }

    #[test]
    fn test_initialize() {
        let paths = test_paths();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(1.into())),
            method: "initialize".to_string(),
            params: Value::Null,
        };
        let resp = handle_initialize(&req);
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        assert_eq!(result["name"], "praxis");
    }

    #[test]
    fn test_unknown_method() {
        let paths = test_paths();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(2.into())),
            method: "nonexistent".to_string(),
            params: Value::Null,
        };
        let resp = handle_request(&paths, &req);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_tools_list() {
        let paths = test_paths();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(3.into())),
            method: "tools/list".to_string(),
            params: Value::Null,
        };
        let resp = handle_tools_list(&paths, &req);
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        // Should at least have the built-in tools
        assert!(tools.len() >= 7);
    }

    #[test]
    fn test_status() {
        let paths = test_paths();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(4.into())),
            method: "status".to_string(),
            params: Value::Null,
        };
        let resp = handle_status(&paths, &req);
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        assert_eq!(result["status"], "running");
    }
}
