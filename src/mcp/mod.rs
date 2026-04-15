/// Model Context Protocol (MCP) support.
///
/// Praxis implements a JSON-RPC 2.0 MCP server that exposes its tool registry
/// as MCP tools. It also provides an MCP client that can consume external MCP
/// servers as additional tool sources.
pub mod client;
pub mod server;
pub mod types;
