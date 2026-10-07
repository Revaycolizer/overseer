#![recursion_limit = "512"]

//! Overseer engine library.
//!
//! The user-facing UI now lives in the standalone Tauri client (`desktop-ui/`),
//! which talks to this engine over the local daemon (`overseer --server`,
//! WebSocket at `/api/ws/chat`). This crate is the headless engine: policy
//! kernel, orchestrator, tools, server, and the terminal (TUI) client.

pub mod backend;
#[path = "db/mod.rs"]
pub mod db;
pub mod engine;
pub mod mcp;
pub mod server;
pub mod skills;
pub mod tui;
