use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::{ActiveTab, App};

pub async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let is_orch = app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst);

    // ── Global shortcuts (always active) ──

    // Ctrl+Left / Ctrl+Right / Shift+Left / Shift+Right — cycle tabs
    if (key.code == KeyCode::Left && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Left && key.modifiers.contains(KeyModifiers::SHIFT))
    {
        app.active_tab = app.active_tab.prev();
        app.activity.bump();
        return Ok(());
    }
    if (key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Right && key.modifiers.contains(KeyModifiers::SHIFT))
    {
        app.active_tab = app.active_tab.next();
        app.activity.bump();
        return Ok(());
    }

    // Alt+1..Alt+9, Alt+0 — jump directly to a tab by number
    if let KeyCode::Char(c) = key.code {
        if key.modifiers.contains(KeyModifiers::ALT) {
            let idx = c.to_digit(10).map(|d| {
                if d == 0 { 9 } else { d as usize - 1 }
            });
            if let Some(i) = idx {
                if i < ActiveTab::ALL.len() {
                    app.active_tab = ActiveTab::from_index(i);
                    if app.active_tab == ActiveTab::Metrics {
                        let _ = app.refresh_metrics().await;
                    }
                    app.activity.bump();
                    return Ok(());
                }
            }
        }
    }

    // F5 — refresh current view
    if key.code == KeyCode::F(5) {
        let _ = app.refresh_conversations().await;
        let _ = app.refresh_metrics().await;
        app.status_message = "Refreshed.".to_string();
        app.activity.bump();
        return Ok(());
    }

    // Ctrl+F — search (placeholder for Phase 5)
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.active_tab = ActiveTab::SearchResults;
        app.status_message = "Search: not yet implemented (Phase 5)".to_string();
        app.activity.bump();
        return Ok(());
    }

    // F1 — help (placeholder for Phase 5)
    if key.code == KeyCode::F(1) {
        app.status_message = "Help: not yet implemented (Phase 5). Available tabs: Chat, Agents, Supervisors, Tools, Workflows, Browser, Computer, Settings, MCP, Skills, Metrics. Use Alt+1..0 to jump.".to_string();
        app.activity.bump();
        return Ok(());
    }

    // Ctrl+N / F2 — new conversation (only in Chat tab)
    if app.active_tab == ActiveTab::Chat
        && (key.code == KeyCode::F(2)
            || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL)))
    {
        let new_id = app.client.add_conversation("New Conversation").await?;
        app.selected_conv_id = Some(new_id);
        app.activity.bump();
        app.input.clear();
        app.input_cursor = 0;
        let _ = app.refresh_conversations().await;
        return Ok(());
    }

    // ── Tab-specific handling ──
    match app.active_tab {
        ActiveTab::Chat => handle_chat_key(app, key, is_orch).await?,
        ActiveTab::Metrics => {
            // Metrics tab: just refresh on any key
            let _ = app.refresh_metrics().await;
        }
        ActiveTab::Agents => {
            handle_agent_keys(app, key).await?;
        }
        _ => {
            // Placeholder tabs — any key shows coming-soon message
            if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
                app.status_message = format!(
                    "{} tab is not yet implemented — coming in a future phase.",
                    app.active_tab.label()
                );
                app.activity.bump();
            }
        }
    }

    Ok(())
}

async fn handle_chat_key(app: &mut App, key: KeyEvent, is_orch: bool) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Tab => {
            if app.input.starts_with('/') {
                let suggestions = crate::engine::commands::get_suggestions(&app.input);
                if let Some(first_match) = suggestions.first() {
                    app.input = first_match.to_string();
                    app.input_cursor = app.input.len();
                    app.activity.bump();
                }
            }
            // Tab in chat cycles to next tab (moved to Ctrl+Right for tab cycle,
            // but bare Tab in chat input should insert spaces or autocomplete)
        }
        KeyCode::Up => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx > 0 {
                    app.selected_conv_id = Some(app.conversations[idx - 1].id);
                    app.refresh_conversations().await?;
                }
            }
        }
        KeyCode::Down => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx + 1 < app.conversations.len() {
                    app.selected_conv_id = Some(app.conversations[idx + 1].id);
                    app.refresh_conversations().await?;
                }
            }
        }
        KeyCode::Enter => {
            if !is_orch {
                if let Some(cmd) = crate::engine::commands::parse(&app.input) {
                    super::command_handler::handle_slash(app, cmd).await?;
                } else {
                    app.send_message().await?;
                }
            }
        }
        KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right
            | KeyCode::Home | KeyCode::End | KeyCode::Char(_) => {
            if !is_orch {
                super::input_handler::app(&key, app);
            }
        }
        KeyCode::Esc => {
            if is_orch {
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

async fn handle_agent_keys(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Up => {
            if let Some(idx) = app.agents.iter().position(|a| Some(a.id) == app.selected_agent_id) {
                if idx > 0 {
                    app.selected_agent_id = Some(app.agents[idx - 1].id);
                    app.activity.bump();
                }
            } else if !app.agents.is_empty() {
                app.selected_agent_id = Some(app.agents[0].id);
            }
        }
        KeyCode::Down => {
            if let Some(idx) = app.agents.iter().position(|a| Some(a.id) == app.selected_agent_id) {
                if idx + 1 < app.agents.len() {
                    app.selected_agent_id = Some(app.agents[idx + 1].id);
                    app.activity.bump();
                }
            } else if !app.agents.is_empty() {
                app.selected_agent_id = Some(app.agents[0].id);
            }
        }
        KeyCode::F(3) => {
            // New agent (placeholder)
            app.status_message = "New Agent: not yet implemented (will open creation form)".to_string();
            app.activity.bump();
        }
        KeyCode::F(2) => {
            // Edit agent (placeholder)
            app.status_message = "Edit Agent: not yet implemented (will open edit form)".to_string();
            app.activity.bump();
        }
        KeyCode::Delete => {
            // Delete agent (placeholder)
            if let Some(id) = app.selected_agent_id {
                app.status_message = format!("Delete agent {}: not yet implemented", id);
                app.activity.bump();
            }
        }
        KeyCode::Esc => {
            // Switch back to chat
            app.active_tab = ActiveTab::Chat;
            app.activity.bump();
        }
        _ => {}
    }
    Ok(())
}
