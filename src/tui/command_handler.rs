use super::app::App;
use crate::engine::commands::SlashCommand;

pub async fn handle_slash(app: &mut App, cmd: SlashCommand) -> anyhow::Result<()> {
    match cmd {
        SlashCommand::Help => {
            app.status_message = "Commands: /new, /export md|json, /search <q>, /model <name>, /regen, /rename <title>, /clear, /help".to_string();
            app.input.clear();
            app.input_cursor = 0;
        }
        SlashCommand::NewConversation => {
            let _ = app.client.add_conversation("New Conversation").await;
            app.input.clear();
            app.input_cursor = 0;
            app.activity.bump();
            app.status_message = "New conversation created.".to_string();
        }
        SlashCommand::Export { format: fmt } => {
            if fmt != "md" && fmt != "json" {
                app.status_message = "Only `/export md` and `/export json` are supported".to_string();
            } else if let Some(conv_id) = app.selected_conv_id {
                let (content, extension) = match fmt.as_str() {
                    "md" => (app.client.export_conversation_markdown(conv_id).await, "md"),
                    "json" => (app.client.export_conversation_json(conv_id).await, "json"),
                    _ => unreachable!(),
                };
                let content = content.unwrap_or_default();
                if !content.is_empty() {
                    let safe = app.conversations.iter()
                        .find(|c| c.id == conv_id)
                        .map(|c| c.title.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect::<String>())
                        .unwrap_or_else(|| format!("conv_{conv_id}"));
                    let filename = format!("{safe}.{extension}");
                    let _ = app.client.export_to_downloads(&filename, &content).await;
                    app.status_message = format!("Exported to Downloads/{filename}");
                }
                app.input.clear();
                app.input_cursor = 0;
            } else {
                app.status_message = "No conversation selected".to_string();
            }
        }
        SlashCommand::Search { query } => {
            let q = query.trim().to_string();
            if q.is_empty() {
                app.status_message = "Please provide a query: /search <query>".to_string();
            } else {
                app.search_query = q.clone();
                match app.client.search_messages(&q).await {
                    Ok(results) => {
                        app.status_message = format!("Found {} matches. ESC to clear.", results.len());
                        app.search_results = Some(results);
                    }
                    Err(e) => app.status_message = format!("Search failed: {e}"),
                }
                app.input.clear();
                app.input_cursor = 0;
            }
        }
        SlashCommand::Model { name } => {
            let m = name.trim().to_string();
            if m.is_empty() {
                app.status_message = "Please specify a model: /model <name>".to_string();
            } else {
                let _ = app.client.set_setting("planner_model", &m).await;
                if let Some(cid) = app.selected_conv_id {
                    let _ = app.client.set_conversation_setting(cid, "model", &m).await;
                }
                app.status_message = format!("Planner model updated to '{}' (per-conversation)", m);
                app.input.clear();
                app.input_cursor = 0;
            }
        }
        SlashCommand::Regenerate => {
            if let Some(last_msg) = app.messages.last().cloned() {
                if last_msg.role == "assistant" {
                    let _ = app.client.delete_message(last_msg.id).await;
                    if let Some(last_user) = app.messages.iter().filter(|m| m.role == "user").last().cloned() {
                        app.input = last_user.content;
                        app.input_cursor = app.input.len();
                        let _ = app.send_message().await;
                    }
                }
            }
        }
        SlashCommand::Rename { title } => {
            if let Some(conv_id) = app.selected_conv_id {
                let _ = app.client.update_conversation_title(conv_id, &title).await;
                        app.status_message = format!("Renamed to '{}'", title);
                app.activity.bump();
            }
        }
        SlashCommand::Clear => {
            app.input.clear();
            app.input_cursor = 0;
        }
        SlashCommand::Quit => app.status_message = "Please press Ctrl+C or 'q' to quit".to_string(),
    }
    Ok(())
}
