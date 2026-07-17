// src/engine/commands.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SlashCommand {
    Help,
    NewConversation,
    Export { format: String },
    Search { query: String },
    Model { name: String },
    Regenerate,
    Rename { title: String },
    Clear,
    Quit,
}

pub fn parse(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    if trimmed == "/help" {
        return Some(SlashCommand::Help);
    }
    if trimmed == "/new" {
        return Some(SlashCommand::NewConversation);
    }
    if trimmed == "/regen" {
        return Some(SlashCommand::Regenerate);
    }
    if trimmed == "/clear" {
        return Some(SlashCommand::Clear);
    }
    if trimmed == "/quit" {
        return Some(SlashCommand::Quit);
    }

    if let Some(arg) = trimmed.strip_prefix("/export ") {
        return Some(SlashCommand::Export { format: arg.trim().to_string() });
    }
    if let Some(arg) = trimmed.strip_prefix("/search ") {
        return Some(SlashCommand::Search { query: arg.trim().to_string() });
    }
    if let Some(arg) = trimmed.strip_prefix("/model ") {
        return Some(SlashCommand::Model { name: arg.trim().to_string() });
    }
    if let Some(arg) = trimmed.strip_prefix("/rename ") {
        return Some(SlashCommand::Rename { title: arg.trim().to_string() });
    }

    None
}

pub fn get_suggestions(input: &str) -> Vec<&'static str> {
    let commands = vec![
        "/help",
        "/new",
        "/export md",
        "/export json",
        "/search ",
        "/model ",
        "/regen",
        "/rename ",
        "/clear",
        "/quit",
    ];
    if input.is_empty() {
        return commands;
    }
    commands.into_iter()
        .filter(|cmd| cmd.starts_with(input))
        .collect()
}

