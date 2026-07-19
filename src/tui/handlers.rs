use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::{ActiveTab, App};

pub async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let is_orch = app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst);

    // ── Global shortcuts ──

    // Ctrl+Left / Ctrl+Right — cycle tabs
    if key.code == KeyCode::Left && (key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SHIFT)) {
        app.active_tab = app.active_tab.prev(); app.activity.bump(); return Ok(());
    }
    if key.code == KeyCode::Right && (key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::SHIFT)) {
        app.active_tab = app.active_tab.next(); app.activity.bump(); return Ok(());
    }

    // Alt+1..9,0 — jump to tab
    if let KeyCode::Char(c) = key.code {
        if key.modifiers.contains(KeyModifiers::ALT) {
            let idx = c.to_digit(10).map(|d| if d == 0 { 9 } else { d as usize - 1 });
            if let Some(i) = idx {
                if i < ActiveTab::ALL.len() {
                    app.active_tab = ActiveTab::from_index(i);
                    refresh_current_tab(app).await;
                    app.activity.bump();
                    return Ok(());
                }
            }
        }
    }

    // F5 — refresh
    if key.code == KeyCode::F(5) {
        refresh_current_tab(app).await;
        app.status_message = "Refreshed.".to_string();
        app.activity.bump();
        return Ok(());
    }

    // Ctrl+F — search
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.active_tab = ActiveTab::SearchResults;
        app.status_message = "Search: Ctrl+F to search messages (coming in Phase 5)".to_string();
        app.activity.bump();
        return Ok(());
    }

    // F1 — help
    if key.code == KeyCode::F(1) {
        let tab_name = app.active_tab.label();
        app.status_message = format!("Help: {} tab. Ctrl+Left/Right=switch tabs, Alt+1..9,0=direct jump, F5=refresh, Q=quit", tab_name);
        app.activity.bump();
        return Ok(());
    }

    // Ctrl+N / F2 — new conversation (Chat only)
    if app.active_tab == ActiveTab::Chat
        && (key.code == KeyCode::F(2) || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL)))
    {
        let new_id = app.client.add_conversation("New Conversation").await?;
        app.selected_conv_id = Some(new_id);
        app.activity.bump();
        app.input.clear(); app.input_cursor = 0;
        let _ = app.refresh_conversations().await;
        return Ok(());
    }

    // ── Tab-specific ──
    match app.active_tab {
        ActiveTab::Chat => handle_chat_key(app, key, is_orch).await?,
        ActiveTab::Metrics => { let _ = app.refresh_metrics().await; }
        ActiveTab::Agents => { nav_list(key, &app.agents, &mut app.selected_agent_id, &mut app.active_tab); app.activity.bump(); }
        ActiveTab::Supervisors => { nav_list(key, &app.supervisors, &mut app.selected_supervisor_id, &mut app.active_tab); app.activity.bump(); }
        ActiveTab::Tools => { nav_list(key, &app.tools, &mut app.selected_tool_id, &mut app.active_tab); app.activity.bump(); }
        ActiveTab::Workflows => { nav_list(key, &app.workflows, &mut app.selected_workflow_id, &mut app.active_tab); app.activity.bump(); }
        ActiveTab::Computer => { handle_computer_key(app, key).await?; }
        ActiveTab::MCP => { handle_mcp_key(app, key).await?; }
        ActiveTab::Skills => { handle_skills_key(app, key).await?; }
        ActiveTab::Settings => {
            if key.code == KeyCode::Esc { app.active_tab = ActiveTab::Chat; app.activity.bump(); }
        }
        ActiveTab::Browser => {}
        _ => {
            if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
                app.status_message = format!("{} tab — coming in a future phase.", app.active_tab.label());
                app.activity.bump();
            }
        }
    }
    Ok(())
}

/// Navigate a list with Up/Down keys. Returns true if the selection changed.
fn nav_list<T>(
    key: KeyEvent,
    items: &[T],
    selected_id: &mut Option<i64>,
    active_tab: &mut ActiveTab,
) where T: HasId {
    match key.code {
        KeyCode::Up => {
            let idx = items.iter().position(|x| Some(x.id()) == *selected_id);
            if let Some(i) = idx { if i > 0 { *selected_id = Some(items[i - 1].id()); } }
            else if !items.is_empty() { *selected_id = Some(items[0].id()); }
        }
        KeyCode::Down => {
            let idx = items.iter().position(|x| Some(x.id()) == *selected_id);
            if let Some(i) = idx { if i + 1 < items.len() { *selected_id = Some(items[i + 1].id()); } }
            else if !items.is_empty() { *selected_id = Some(items[0].id()); }
        }
        KeyCode::Esc => { *active_tab = ActiveTab::Chat; }
        _ => {}
    }
}

trait HasId { fn id(&self) -> i64; }
impl HasId for crate::engine::utils::discovery::AgentInfo { fn id(&self) -> i64 { self.id } }
impl HasId for crate::db::Supervisor { fn id(&self) -> i64 { self.id } }
impl HasId for crate::engine::utils::discovery::ToolInfo { fn id(&self) -> i64 { self.id } }
impl HasId for crate::backend::workflows::models::Workflow { fn id(&self) -> i64 { self.id } }

async fn handle_computer_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::F(2) => {
            let _ = app.client.activate_computer_session(true, "").await;
            app.computer_actions.push(super::app::ActionEntry { status: "info".into(), tool: "system".into(), detail: "Session activated".into() });
            let _ = app.refresh_computer().await;
            app.status_message = "Computer control activated.".to_string();
        }
        KeyCode::F(3) => {
            let _ = app.client.activate_computer_session(false, "").await;
            app.computer_actions.push(super::app::ActionEntry { status: "info".into(), tool: "system".into(), detail: "Session deactivated".into() });
            let _ = app.refresh_computer().await;
            app.status_message = "Computer control deactivated.".to_string();
        }
        KeyCode::F(4) => {
            let _ = app.client.approve_pending_computer_action().await;
            app.computer_actions.push(super::app::ActionEntry { status: "approved".into(), tool: "user".into(), detail: "Action approved".into() });
            app.status_message = "Action approved.".to_string();
        }
        KeyCode::F(5) => {
            let _ = app.client.reject_pending_computer_action().await;
            app.computer_actions.push(super::app::ActionEntry { status: "rejected".into(), tool: "user".into(), detail: "Action rejected".into() });
            app.status_message = "Action rejected.".to_string();
        }
        KeyCode::F(6) => {
            app.computer_actions.push(super::app::ActionEntry { status: "emergency".into(), tool: "system".into(), detail: "EMERGENCY STOP".into() });
            app.status_message = "EMERGENCY STOP — all actions blocked.".to_string();
        }
        KeyCode::Esc => { app.active_tab = ActiveTab::Chat; app.activity.bump(); }
        _ => {}
    }
    Ok(())
}

async fn handle_mcp_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::F(2) => {
            // Connect to MCP server
            let url = app.mcp_server_url.clone();
            let mut client = crate::mcp::client::McpClient::new(&url);
            match client.list_tools().await {
                Ok(tools) => {
                    app.mcp_tools = tools;
                    if let Ok(resources) = client.list_resources().await {
                        app.mcp_resources = resources;
                    }
                    app.mcp_connected = true;
                    app.status_message = format!("Connected to MCP server at {}", url);
                }
                Err(e) => {
                    app.status_message = format!("Failed to connect: {}", e);
                }
            }
            app.activity.bump();
        }
        KeyCode::F(3) => {
            app.mcp_connected = false;
            app.mcp_tools.clear();
            app.mcp_resources.clear();
            app.status_message = "Disconnected from MCP server.".to_string();
            app.activity.bump();
        }
        KeyCode::Esc => { app.active_tab = ActiveTab::Chat; app.activity.bump(); }
        _ => {}
    }
    Ok(())
}

async fn handle_skills_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::F(2) => {
            // Install placeholder (DB-backed install coming when backend supports it)
            app.status_message = "Skill install: requires database access from TUI context (Phase 5).".to_string();
            app.activity.bump();
        }
        KeyCode::Esc => { app.active_tab = ActiveTab::Chat; app.activity.bump(); }
        _ => {}
    }
    Ok(())
}

async fn refresh_current_tab(app: &mut App) {
    match app.active_tab {
        ActiveTab::Agents => { let _ = app.refresh_agents().await; }
        ActiveTab::Supervisors => { let _ = app.refresh_supervisors().await; }
        ActiveTab::Tools => { let _ = app.refresh_tools().await; }
        ActiveTab::Workflows => { let _ = app.refresh_workflows().await; }
        ActiveTab::Computer => { let _ = app.refresh_computer().await; }
        ActiveTab::Settings => { let _ = app.refresh_settings_map().await; }
        ActiveTab::MCP => {
            if app.mcp_connected {
                let url = app.mcp_server_url.clone();
                let mut client = crate::mcp::client::McpClient::new(&url);
                if let Ok(tools) = client.list_tools().await { app.mcp_tools = tools; }
                if let Ok(resources) = client.list_resources().await { app.mcp_resources = resources; }
            }
        }
        ActiveTab::Metrics => { let _ = app.refresh_metrics().await; }
        ActiveTab::Chat => { let _ = app.refresh_conversations().await; }
        _ => {}
    }
}

async fn handle_chat_key(app: &mut App, key: KeyEvent, is_orch: bool) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Tab => {
            if app.input.starts_with('/') {
                if let Some(f) = crate::engine::commands::get_suggestions(&app.input).first() {
                    app.input = f.to_string(); app.input_cursor = app.input.len(); app.activity.bump();
                }
            }
        }
        KeyCode::Up => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx > 0 { app.selected_conv_id = Some(app.conversations[idx - 1].id); app.refresh_conversations().await?; }
            }
        }
        KeyCode::Down => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx + 1 < app.conversations.len() { app.selected_conv_id = Some(app.conversations[idx + 1].id); app.refresh_conversations().await?; }
            }
        }
        KeyCode::Enter => {
            if !is_orch {
                if let Some(cmd) = crate::engine::commands::parse(&app.input) {
                    super::command_handler::handle_slash(app, cmd).await?;
                } else { app.send_message().await?; }
            }
        }
        KeyCode::Backspace | KeyCode::Delete | KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End | KeyCode::Char(_) => {
            if !is_orch { super::input_handler::app(&key, app); }
        }
        KeyCode::Esc => {
            if is_orch { app.cancel_current_orchestrate(); app.status_message = "Cancelling…".to_string(); }
            else if app.search_results.is_some() { app.search_results = None; app.search_query.clear(); app.status_message = "Search cleared".to_string(); app.activity.bump(); }
        }
        _ => {}
    }
    if app.input.starts_with('/') {
        let s = crate::engine::commands::get_suggestions(&app.input);
        app.status_message = if s.is_empty() { "No matching command".into() } else { format!("Suggestions: {} (TAB to autocomplete)", s.join(", ")) };
    }
    Ok(())
}
