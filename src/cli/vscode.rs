use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde_json::json;

use crate::paths::{PraxisPaths, default_data_dir};

#[derive(Debug, Args)]
pub struct VscodeArgs {
    #[command(subcommand)]
    command: VscodeCommand,
}

#[derive(Debug, Subcommand)]
enum VscodeCommand {
    /// Generate .vscode/tasks.json for this workspace.
    Init(VscodeInitArgs),
}

#[derive(Debug, Args)]
struct VscodeInitArgs {
    /// Workspace root (defaults to current directory).
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// Praxis serve host (used in status task).
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Praxis serve port.
    #[arg(long, default_value_t = 8787)]
    port: u16,
}

pub(crate) fn handle_vscode(
    data_dir_override: Option<PathBuf>,
    args: VscodeArgs,
) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    match args.command {
        VscodeCommand::Init(a) => vscode_init(&paths, a),
    }
}

fn vscode_init(paths: &PraxisPaths, args: VscodeInitArgs) -> Result<String> {
    let workspace = args
        .workspace
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let vscode_dir = workspace.join(".vscode");
    fs::create_dir_all(&vscode_dir)
        .with_context(|| format!("failed to create {}", vscode_dir.display()))?;

    let tasks_path = vscode_dir.join("tasks.json");
    let data_dir = paths.data_dir.display().to_string();
    let status_url = format!("http://{}:{}/summary", args.host, args.port);

    let tasks = json!({
        "version": "2.0.0",
        "tasks": [
            {
                "label": "Praxis: Run once",
                "type": "shell",
                "command": "praxis",
                "args": ["--data-dir", data_dir, "run", "--once"],
                "group": "build",
                "presentation": { "reveal": "always", "panel": "shared" },
                "problemMatcher": []
            },
            {
                "label": "Praxis: Status",
                "type": "shell",
                "command": "praxis",
                "args": ["--data-dir", data_dir, "status"],
                "group": "test",
                "presentation": { "reveal": "always", "panel": "shared" },
                "problemMatcher": []
            },
            {
                "label": "Praxis: Serve dashboard",
                "type": "shell",
                "command": "praxis",
                "args": ["--data-dir", data_dir, "serve", "--host", args.host, "--port", args.port.to_string()],
                "isBackground": true,
                "presentation": { "reveal": "always", "panel": "dedicated" },
                "problemMatcher": [
                    {
                        "pattern": { "regexp": "^$" },
                        "background": {
                            "activeOnStart": true,
                            "beginsPattern": "^$",
                            "endsPattern": "Listening on"
                        }
                    }
                ]
            },
            {
                "label": "Praxis: Fetch status JSON",
                "type": "shell",
                "command": "curl",
                "args": ["-s", status_url],
                "group": "test",
                "presentation": { "reveal": "always", "panel": "shared" },
                "problemMatcher": []
            },
            {
                "label": "Praxis: Doctor",
                "type": "shell",
                "command": "praxis",
                "args": ["--data-dir", data_dir, "doctor"],
                "group": "test",
                "presentation": { "reveal": "always", "panel": "shared" },
                "problemMatcher": []
            },
            {
                "label": "Praxis: Git sync",
                "type": "shell",
                "command": "praxis",
                "args": ["--data-dir", data_dir, "git", "commit"],
                "group": "build",
                "presentation": { "reveal": "always", "panel": "shared" },
                "problemMatcher": []
            }
        ]
    });

    let content = serde_json::to_string_pretty(&tasks).context("failed to serialize tasks.json")?;
    fs::write(&tasks_path, &content)
        .with_context(|| format!("failed to write {}", tasks_path.display()))?;

    Ok(format!(
        "vscode: wrote {}\ntasks: 6\ndashboard: http://{}:{}",
        tasks_path.display(),
        args.host,
        args.port
    ))
}
