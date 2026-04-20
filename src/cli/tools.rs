use std::{collections::HashMap, fmt::Write as _, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};

use crate::{
    storage::{
        ApprovalStatus, ApprovalStore, NewApprovalRequest, SessionStore, SqliteSessionStore,
        StoredApprovalRequest,
    },
    tools::{
        FileToolRegistry, SecurityPolicy, ToolKind, ToolManifest, ToolRegistry, build_payload,
        sync_capabilities,
    },
};

use super::core::load_initialized_config;

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("invalid param '{s}': expected key=value"))
}

#[derive(Debug, Args)]
pub struct ToolsArgs {
    #[command(subcommand)]
    command: ToolCommands,
}

#[derive(Debug, Subcommand)]
enum ToolCommands {
    List,
    Register(RegisterToolArgs),
    Request(RequestToolArgs),
}

#[derive(Debug, Args)]
struct RegisterToolArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    description: String,
    #[arg(long, value_enum)]
    kind: ToolKindArg,
    #[arg(long, default_value_t = 2)]
    level: u8,
    #[arg(long)]
    approval: bool,
    #[arg(long)]
    rehearsal: bool,
    #[arg(long = "allow-path")]
    allow_paths: Vec<String>,
}

#[derive(Debug, Args)]
struct RequestToolArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    summary: String,
    #[arg(long, default_value = "operator")]
    requested_by: String,
    #[arg(long = "write-path")]
    write_paths: Vec<String>,
    #[arg(long)]
    append_text: Option<String>,
    /// Key=value params for shell-exec, file-read, web-fetch, git-query.
    #[arg(long = "param", value_parser = parse_key_val)]
    params: Vec<(String, String)>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ToolKindArg {
    Internal,
    Shell,
    Http,
}

pub(super) fn handle_tools(data_dir_override: Option<PathBuf>, args: ToolsArgs) -> Result<String> {
    match args.command {
        ToolCommands::List => handle_list(data_dir_override),
        ToolCommands::Register(args) => handle_register(data_dir_override, args),
        ToolCommands::Request(args) => handle_request(data_dir_override, args),
    }
}

fn handle_list(data_dir_override: Option<PathBuf>) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    FileToolRegistry.validate(&paths)?;
    let tools = FileToolRegistry.list(&paths)?;
    if tools.is_empty() {
        return Ok("tools: none".to_string());
    }

    Ok(tools
        .into_iter()
        .map(|tool| tool.summary_line())
        .collect::<Vec<_>>()
        .join("\n"))
}

fn handle_register(data_dir_override: Option<PathBuf>, args: RegisterToolArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;
    let manifest = ToolManifest {
        name: args.name,
        description: args.description,
        kind: args.kind.into(),
        required_level: args.level,
        requires_approval: args.approval,
        rehearsal_required: args.rehearsal,
        allowed_paths: args.allow_paths,
        allowed_read_paths: Vec::new(),
        path: None,
        args: Vec::new(),
        timeout_secs: None,
        endpoint: None,
        method: None,
        headers: Vec::new(),
        body: None,
        allowed_vault_keys: None,
    };

    let path = FileToolRegistry.register(&paths, &manifest)?;
    sync_capabilities(&FileToolRegistry, &store, &paths)?;
    Ok(format!(
        "tool: registered\nname: {}\nmanifest: {}",
        manifest.name,
        path.display()
    ))
}

fn handle_request(data_dir_override: Option<PathBuf>, args: RequestToolArgs) -> Result<String> {
    let (config, paths) = load_initialized_config(data_dir_override)?;
    let manifest = FileToolRegistry
        .get(&paths, &args.name)?
        .with_context(|| format!("tool {} not found", args.name))?;
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.initialize()?;

    let status = if manifest.requires_approval {
        ApprovalStatus::Pending
    } else {
        ApprovalStatus::Approved
    };
    if manifest.name == "praxis-data-write" && args.write_paths.is_empty() {
        bail!("tool {} requires at least one --write-path", manifest.name);
    }
    let request = NewApprovalRequest {
        tool_name: manifest.name.clone(),
        summary: args.summary,
        requested_by: args.requested_by,
        write_paths: args.write_paths,
        payload_json: build_payload(
            &manifest,
            args.append_text,
            args.params.into_iter().collect::<HashMap<_, _>>(),
        )?,
        status,
    };
    SecurityPolicy.validate_request(
        &config,
        &paths,
        &manifest,
        &StoredApprovalRequest {
            id: 0,
            tool_name: request.tool_name.clone(),
            summary: request.summary.clone(),
            requested_by: request.requested_by.clone(),
            write_paths: request.write_paths.clone(),
            payload_json: request.payload_json.clone(),
            status: request.status,
            status_note: None,
            created_at: String::new(),
            updated_at: String::new(),
        },
    )?;
    let stored = store.queue_approval(&request)?;
    sync_capabilities(&FileToolRegistry, &store, &paths)?;

    let mut output = String::new();
    writeln!(output, "request: {}", stored.status.as_str())?;
    writeln!(output, "id: {}", stored.id)?;
    writeln!(output, "tool: {}", stored.tool_name)?;
    write!(output, "paths: {}", stored.write_paths.join(","))?;
    Ok(output)
}

impl From<ToolKindArg> for ToolKind {
    fn from(value: ToolKindArg) -> Self {
        match value {
            ToolKindArg::Internal => ToolKind::Internal,
            ToolKindArg::Shell => ToolKind::Shell,
            ToolKindArg::Http => ToolKind::Http,
        }
    }
}
