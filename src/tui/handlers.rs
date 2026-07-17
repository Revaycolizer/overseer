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
            app.active_tab = match app.active_tab {
                ActiveTab::Chat => ActiveTab::Metrics,
                ActiveTab::Metrics => {
                    let _ = app.refresh_metrics().await;
                    ActiveTab::Chat
                }
            };
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
                let trimmed = app.input.trim();
                if trimmed == "/help" {
                    app.status_message = "Commands: /export md, /help".to_string();
                } else if let Some(arg) = trimmed.strip_prefix("/export ") {
                    let fmt = arg.trim();
                    if fmt != "md" {
                        app.status_message = "Only `/export md` is supported".to_string();
                    } else if let Some(conv_id) = app.selected_conv_id {
                        let content = match app.client.export_conversation_markdown(conv_id).await {
                            Ok(s) => s,
                            Err(e) => {
                                app.status_message = format!("Export failed: {e}");
                                String::new()
                            }
                        };
                        if !content.is_empty() {
                            let safe = app
                                .conversations
                                .iter()
                                .find(|c| c.id == conv_id)
                                .map(|c| c.title.clone())
                                .unwrap_or_else(|| format!("conv_{conv_id}"));
                            let safe = safe
                                .chars()
                                .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
                                .collect::<String>();
                            let filename = format!("{safe}.md");
                            let _ = app.client.export_to_downloads(&filename, &content).await;
                            app.status_message = format!("Exported to Downloads/{filename}");
                        }
                    } else {
                        app.status_message = "No conversation selected".to_string();
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
            }
        }
        _ => {}
    }
    Ok(())
}
