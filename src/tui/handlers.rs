use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::{ActiveTab, App};

pub async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let is_orch = app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst);

    if key.code == KeyCode::F(2) || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL)) {
        let new_id = app.client.add_conversation("New Conversation").await?;
        // Immediately anchor selection to the new conversation's ID —
        // before any refresh that might re-sort the list. This prevents
        // the race where refresh_conversations() returns before the INSERT
        // is visible, causing unwrap_or(0) to silently select the wrong
        // conversation.
        app.selected_conv_id = Some(new_id);
        app.activity.bump();
        app.input.clear();
        app.input_cursor = 0;
        let _ = app.refresh_conversations().await;
        return Ok(());
    }

    match key.code {
        KeyCode::Tab => {
            if app.active_tab == ActiveTab::Chat && app.input.starts_with('/') {
                let suggestions = crate::engine::commands::get_suggestions(&app.input);
                if let Some(first_match) = suggestions.first() {
                    app.input = first_match.to_string();
                    app.input_cursor = app.input.len();
                    app.activity.bump();
                }
            } else {
                app.active_tab = match app.active_tab {
                    ActiveTab::Chat => ActiveTab::Metrics,
                    ActiveTab::Metrics => {
                        let _ = app.refresh_metrics().await;
                        ActiveTab::Chat
                    }
                };
            }
        }
        KeyCode::Up => {
            if app.active_tab == ActiveTab::Chat {
                if let Some(idx) = app.selected_conv_idx() {
                    if idx > 0 {
                        app.selected_conv_id = Some(app.conversations[idx - 1].id);
                        app.refresh_conversations().await?;
                    }
                }
            }
        }
        KeyCode::Down => {
            if app.active_tab == ActiveTab::Chat {
                if let Some(idx) = app.selected_conv_idx() {
                    if idx + 1 < app.conversations.len() {
                        app.selected_conv_id = Some(app.conversations[idx + 1].id);
                        app.refresh_conversations().await?;
                    }
                }
            }
        }
        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                // V4.17.10: slash command dispatch. The send
                // message is the default; `/export md` and `/help`
                // are intercepted here so the input doesn't get
                // treated as a chat query. New commands can be added
                // by extending this match.
                if let Some(cmd) = crate::engine::commands::parse(&app.input) {
                    match cmd {
                        crate::engine::commands::SlashCommand::Help => {
                            app.status_message = "Commands: /new, /export md|json, /search <q>, /model <name>, /regen, /rename <title>, /clear, /help".to_string();
                            app.input.clear();
                            app.input_cursor = 0;
                        }
                        crate::engine::commands::SlashCommand::NewConversation => {
                            let new_id = app.client.add_conversation("New Conversation").await?;
                            app.selected_conv_id = Some(new_id);
                            app.input.clear();
                            app.input_cursor = 0;
                            let _ = app.refresh_conversations().await;
                        }
                        crate::engine::commands::SlashCommand::Export { format } => {
                            let fmt = format.trim();
                            if fmt != "md" && fmt != "json" {
                                app.status_message = "Only `/export md` and `/export json` are supported".to_string();
                            } else if let Some(conv_id) = app.selected_conv_id {
                                let content = match fmt {
                                    "md" => app.client.export_conversation_markdown(conv_id).await,
                                    "json" => app.client.export_conversation_json(conv_id).await,
                                    _ => unreachable!(),
                                };
                                let content = content.unwrap_or_default();
                                if !content.is_empty() {
                                    let safe = app.conversations.iter().find(|c| c.id == conv_id).map(|c| c.title.clone()).unwrap_or_else(|| format!("conv_{conv_id}"));
                                    let safe = safe.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect::<String>();
                                    let filename = format!("{safe}.{fmt}");
                                    let _ = app.client.export_to_downloads(&filename, &content).await;
                                    app.status_message = format!("Exported to Downloads/{filename}");
                                }
                            } else {
                                app.status_message = "No conversation selected".to_string();
                            }
                            app.input.clear();
                            app.input_cursor = 0;
                        }
                        crate::engine::commands::SlashCommand::Search { query } => {
                            let q = query.trim().to_string();
                            if q.is_empty() {
                                app.status_message = "Please provide a query: /search <query>".to_string();
                            } else {
                                app.search_query = q.clone();
                                match app.client.search_messages(&q).await {
                                    Ok(results) => {
                                        app.status_message = format!("Found {} matches. ESC to clear.", results.len());
                                        app.search_results = Some(results);
                                        app.input.clear();
                                        app.input_cursor = 0;
                                        app.activity.bump();
                                    }
                                    Err(e) => {
                                        app.status_message = format!("Search failed: {}", e);
                                    }
                                }
                            }
                        }
                        crate::engine::commands::SlashCommand::Model { name } => {
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
                        crate::engine::commands::SlashCommand::Regenerate => {
                            if let Some(last_msg) = app.messages.last().cloned() {
                                if last_msg.role == "assistant" {
                                    let _ = app.client.delete_message(last_msg.id).await;
                                    if let Some(last_user) = app.messages.iter().filter(|m| m.role == "user").last().cloned() {
                                        app.input = last_user.content;
                                        app.input_cursor = app.input.len();
                                        let _ = app.send_message().await;
                                    }
                                }
                            } else {
                                app.input.clear();
                                app.input_cursor = 0;
                            }
                        }
                        crate::engine::commands::SlashCommand::Rename { title } => {
                            let t = title.trim().to_string();
                            if let Some(conv_id) = app.selected_conv_id {
                                if !t.is_empty() && app.client.update_conversation_title(conv_id, &t).await.is_ok() {
                                    app.status_message = format!("Conversation renamed to '{}'", t);
                                    let _ = app.refresh_conversations().await;
                                }
                            }
                            app.input.clear();
                            app.input_cursor = 0;
                        }
                        crate::engine::commands::SlashCommand::Clear => {
                            app.input.clear();
                            app.input_cursor = 0;
                        }
                        crate::engine::commands::SlashCommand::Quit => {
                            app.input.clear();
                            app.input_cursor = 0;
                            app.status_message = "Please press Ctrl+C or 'q' to quit".to_string();
                        }
                    }
                } else {
                    app.send_message().await?;
                }
            }
        }
        KeyCode::Left => {
            if app.active_tab == ActiveTab::Chat && app.input_cursor > 0 {
                // Move cursor back over the previous UTF-8 char.
                let prev = app
                    .input
                    .char_indices()
                    .rfind(|(i, _)| *i < app.input_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                app.input_cursor = prev;
            }
        }
        KeyCode::Right => {
            if app.active_tab == ActiveTab::Chat && app.input_cursor < app.input.len() {
                // Move cursor forward over the next UTF-8 char.
                let next = app
                    .input
                    .char_indices()
                    .find(|(i, _)| *i > app.input_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input_cursor = next;
            }
        }
        KeyCode::Home => {
            if app.active_tab == ActiveTab::Chat {
                app.input_cursor = 0;
            }
        }
        KeyCode::End => {
            if app.active_tab == ActiveTab::Chat {
                app.input_cursor = app.input.len();
            }
        }
        KeyCode::Delete => {
            if app.active_tab == ActiveTab::Chat && app.input_cursor < app.input.len() {
                // Delete the char under the cursor (forward delete).
                let next = app
                    .input
                    .char_indices()
                    .find(|(i, _)| *i > app.input_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.replace_range(app.input_cursor..next, "");
            }
        }
        KeyCode::Backspace => {
            if app.active_tab == ActiveTab::Chat && app.input_cursor > 0 {
                // Delete the char before the cursor.
                let prev = app
                    .input
                    .char_indices()
                    .rfind(|(i, _)| *i < app.input_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let cur = app
                    .input
                    .char_indices()
                    .find(|(i, _)| *i > prev)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
                app.input.replace_range(prev..cur, "");
                app.input_cursor = prev;
            }
        }
        KeyCode::Char(c) => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                // Insert at cursor position (UTF-8 safe).
                app.input.insert(app.input_cursor, c);
                app.input_cursor += c.len_utf8();
            }
        }
        // V4.17.14: Esc cancels the current in-flight orchestrate
        // task (drops the underlying HTTP stream to Ollama). The
        // spawn itself sets is_orch = false when it unwinds.
        KeyCode::Esc => {
            if app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst) {
                app.cancel_current_orchestrate();
                app.status_message = "Cancelling…".to_string();
            } else if app.search_results.is_some() {
                app.search_results = None;
                app.search_query.clear();
                app.status_message = "Search cleared".to_string();
                app.activity.bump();
            }
        }
        _ => {}
    }
    if app.input.starts_with('/') {
        let suggestions = crate::engine::commands::get_suggestions(&app.input);
        if !suggestions.is_empty() {
            app.status_message = format!("Suggestions: {} (TAB to autocomplete)", suggestions.join(", "));
        } else {
            app.status_message = "No matching command".to_string();
        }
    }
    Ok(())
}
