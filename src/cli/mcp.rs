use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::mcp::server::serve_stdio;

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Run the MCP server over stdio transport.
    ///
    /// Add to Claude Desktop config:
    ///
    /// ```json
    /// {
    ///   "mcpServers": {
    ///     "praxis": {
    ///       "command": "praxis",
    ///       "args": ["mcp", "serve"]
    ///     }
    ///   }
    /// }
    /// ```
    Serve,
}

pub fn handle_mcp(data_dir_override: Option<PathBuf>, args: McpArgs) -> Result<String> {
    match args.command {
        McpCommand::Serve => {
            serve_stdio(data_dir_override)?;
            Ok("mcp server exited".to_string())
        }
    }
}
