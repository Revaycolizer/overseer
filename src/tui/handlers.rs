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
                    ActiveTab::Metrics => ActiveTab::Browser,
                    ActiveTab::Browser => {
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
                if let Some(cmd) = crate::engine::commands::parse(&app.input) {
                    super::command_handler::handle_slash(app, cmd).await?;
                } else {
                    app.send_message().await?;
                }
            }
        }
        KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right
            | KeyCode::Home | KeyCode::End | KeyCode::Char(_) => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                super::input_handler::app(&key, app);
            }
        }
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
