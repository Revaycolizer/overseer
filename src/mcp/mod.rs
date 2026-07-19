//! Model Context Protocol (MCP) implementation.
//!
//! MCP is a JSON-RPC 2.0 based protocol for AI models to discover and invoke
//! tools, read resources, and use prompt templates exposed by a server.
//!
//! This module provides both a **server** (exposing Overseer's agents, tools,
//! and supervisors as MCP resources) and a **client** (for the TUI to connect
//! to external MCP servers).

pub mod server;
pub mod client;
pub mod types;
