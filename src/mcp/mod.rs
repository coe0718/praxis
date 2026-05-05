//! Model Context Protocol (MCP) support.
//!
//! MCP is an open protocol standardising how LLM agents expose tools and
//! resources.  Praxis implements both sides:
//!
//! - **Server** (`mcp serve`): Exposes Praxis tools to external MCP clients
//!   (Claude Desktop, Cursor, etc.) via stdio transport.
//! - **Client** (future): Connects to external MCP servers and registers their
//!   tools in the Praxis tool registry.
//!
//! Reference: https://modelcontextprotocol.io

pub(crate) mod client;
pub(crate) mod server;
pub(crate) mod types;

pub use client::{McpClient, McpServerConfig};
pub use types::*;
