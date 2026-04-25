//! Agent-to-Agent (A2A) protocol support.
//!
//! Implements Google's A2A spec for cross-agent task delegation.
//!
//! - **Client** (`A2aClient`): Delegates tasks to remote agents.
//! - **Types** (`types::*`): Agent cards, task lifecycle, messages, artifacts.

pub mod client;
pub mod types;

pub use client::A2aClient;
pub use types::*;
