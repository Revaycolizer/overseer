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
                app.send_message().await?;
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
