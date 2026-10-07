use super::app::{ActiveTab, App};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let is_orch = app
        .is_orchestrating
        .load(std::sync::atomic::Ordering::SeqCst);

    // ── Global shortcuts ──

    if (key.code == KeyCode::Char('o') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::F(6)
            && matches!(app.active_tab, ActiveTab::Chat | ActiveTab::Settings))
    {
        app.active_tab = ActiveTab::Settings;
        app.refresh_settings_map().await?;
        app.refresh_models().await;
        app.status_message = "Ollama models: Up/Down select, Enter apply, Esc back".into();
        return Ok(());
    }
    if key.code == KeyCode::BackTab || (key.code == KeyCode::Tab && !app.input.starts_with('/')) {
        app.active_tab = if key.code == KeyCode::BackTab {
            app.active_tab.prev()
        } else {
            app.active_tab.next()
        };
        refresh_current_tab(app).await;
        return Ok(());
    }

    // Ctrl+Left / Ctrl+Right — cycle tabs
    if key.code == KeyCode::Left
        && (key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::SHIFT))
    {
        app.active_tab = app.active_tab.prev();
        refresh_current_tab(app).await;
        app.activity.bump();
        return Ok(());
    }
    if key.code == KeyCode::Right
        && (key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::SHIFT))
    {
        app.active_tab = app.active_tab.next();
        refresh_current_tab(app).await;
        app.activity.bump();
        return Ok(());
    }

    // Alt+1..9,0 — jump to tab
    if let KeyCode::Char(c) = key.code {
        if key.modifiers.contains(KeyModifiers::ALT) {
            let idx = c
                .to_digit(10)
                .map(|d| if d == 0 { 9 } else { d as usize - 1 });
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

    // Ctrl+F — open universal search
    if key.code == KeyCode::Char('f') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.show_search = !app.show_search;
        if app.show_search {
            app.search_query.clear();
            app.search_results = None;
        }
        app.activity.bump();
        return Ok(());
    }

    // F1 — help overlay
    if key.code == KeyCode::F(1) {
        app.show_help = !app.show_help;
        if !app.show_help {
            app.status_message = "Help closed.".to_string();
        }
        app.activity.bump();
        return Ok(());
    }

    // If help is open, only Esc closes it
    if app.show_help {
        if key.code == KeyCode::Esc || key.code == KeyCode::F(1) {
            app.show_help = false;
            app.status_message = "Help closed.".to_string();
            app.activity.bump();
        }
        return Ok(());
    }

    // If search is open, route all keys to search input
    if app.show_search {
        return handle_search_key(app, key).await;
    }

    // Ctrl+N / F2 — new conversation (Chat only)
    if app.active_tab == ActiveTab::Chat
        && (key.code == KeyCode::F(2)
            || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL)))
    {
        let new_id = app.client.add_conversation("New Conversation").await?;
        app.selected_conv_id = Some(new_id);
        app.activity.bump();
        app.input.clear();
        app.input_cursor = 0;
        app.chat_scroll = 0;
        let _ = app.refresh_conversations().await;
        return Ok(());
    }

    // ── Tab-specific ──
    match app.active_tab {
        ActiveTab::Chat => handle_chat_key(app, key, is_orch).await?,
        ActiveTab::Metrics => {
            let _ = app.refresh_metrics().await;
        }
        ActiveTab::Agents => {
            nav_list(
                key,
                &app.agents,
                &mut app.selected_agent_id,
                &mut app.active_tab,
            );
            app.activity.bump();
        }
        ActiveTab::Supervisors => {
            nav_list(
                key,
                &app.supervisors,
                &mut app.selected_supervisor_id,
                &mut app.active_tab,
            );
            app.activity.bump();
        }
        ActiveTab::Tools => {
            nav_list(
                key,
                &app.tools,
                &mut app.selected_tool_id,
                &mut app.active_tab,
            );
            app.activity.bump();
        }
        ActiveTab::Workflows => {
            nav_list(
                key,
                &app.workflows,
                &mut app.selected_workflow_id,
                &mut app.active_tab,
            );
            app.activity.bump();
        }
        ActiveTab::Computer => {
            handle_computer_key(app, key).await?;
        }
        ActiveTab::MCP => {
            handle_mcp_key(app, key).await?;
        }
        ActiveTab::Skills => {
            handle_skills_key(app, key).await?;
        }
        ActiveTab::Settings => {
            match key.code {
                KeyCode::Up => app.selected_model = app.selected_model.saturating_sub(1),
                KeyCode::Down => {
                    app.selected_model =
                        (app.selected_model + 1).min(app.available_models.len().saturating_sub(1))
                }
                KeyCode::Enter => {
                    if let Some(model) = app.available_models.get(app.selected_model).cloned() {
                        match app.select_model(&model).await {
                            Ok(()) => app.active_tab = ActiveTab::Chat,
                            Err(e) => app.status_message = format!("Model selection failed: {e}"),
                        }
                    }
                }
                KeyCode::Esc => app.active_tab = ActiveTab::Chat,
                _ => {}
            }
            app.activity.bump();
        }
        ActiveTab::Browser => {}
        _ => {
            if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
                app.status_message =
                    format!("{} tab — coming in a future phase.", app.active_tab.label());
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
) where
    T: HasId,
{
    match key.code {
        KeyCode::Up => {
            let idx = items.iter().position(|x| Some(x.id()) == *selected_id);
            if let Some(i) = idx {
                if i > 0 {
                    *selected_id = Some(items[i - 1].id());
                }
            } else if !items.is_empty() {
                *selected_id = Some(items[0].id());
            }
        }
        KeyCode::Down => {
            let idx = items.iter().position(|x| Some(x.id()) == *selected_id);
            if let Some(i) = idx {
                if i + 1 < items.len() {
                    *selected_id = Some(items[i + 1].id());
                }
            } else if !items.is_empty() {
                *selected_id = Some(items[0].id());
            }
        }
        KeyCode::Esc => {
            *active_tab = ActiveTab::Chat;
        }
        _ => {}
    }
}

trait HasId {
    fn id(&self) -> i64;
}
impl HasId for crate::engine::utils::discovery::AgentInfo {
    fn id(&self) -> i64 {
        self.id
    }
}
impl HasId for crate::db::Supervisor {
    fn id(&self) -> i64 {
        self.id
    }
}
impl HasId for crate::engine::utils::discovery::ToolInfo {
    fn id(&self) -> i64 {
        self.id
    }
}
impl HasId for crate::backend::workflows::models::Workflow {
    fn id(&self) -> i64 {
        self.id
    }
}

async fn handle_computer_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::F(2) => {
            let _ = app.client.activate_computer_session(true, "").await;
            app.computer_actions.push(super::app::ActionEntry {
                status: "info".into(),
                tool: "system".into(),
                detail: "Session activated".into(),
            });
            let _ = app.refresh_computer().await;
            app.status_message = "Computer control activated.".to_string();
        }
        KeyCode::F(3) => {
            let _ = app.client.activate_computer_session(false, "").await;
            app.computer_actions.push(super::app::ActionEntry {
                status: "info".into(),
                tool: "system".into(),
                detail: "Session deactivated".into(),
            });
            let _ = app.refresh_computer().await;
            app.status_message = "Computer control deactivated.".to_string();
        }
        KeyCode::F(4) => {
            let _ = app.client.approve_pending_computer_action().await;
            app.computer_actions.push(super::app::ActionEntry {
                status: "approved".into(),
                tool: "user".into(),
                detail: "Action approved".into(),
            });
            app.status_message = "Action approved.".to_string();
        }
        KeyCode::F(5) => {
            let _ = app.client.reject_pending_computer_action().await;
            app.computer_actions.push(super::app::ActionEntry {
                status: "rejected".into(),
                tool: "user".into(),
                detail: "Action rejected".into(),
            });
            app.status_message = "Action rejected.".to_string();
        }
        KeyCode::F(6) => {
            app.computer_actions.push(super::app::ActionEntry {
                status: "emergency".into(),
                tool: "system".into(),
                detail: "EMERGENCY STOP".into(),
            });
            app.status_message = "EMERGENCY STOP — all actions blocked.".to_string();
        }
        KeyCode::Esc => {
            app.active_tab = ActiveTab::Chat;
            app.activity.bump();
        }
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
        KeyCode::Esc => {
            app.active_tab = ActiveTab::Chat;
            app.activity.bump();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_skills_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::F(2) => {
            if let Some(ref db) = app.db {
                let registry = crate::skills::registry::SkillRegistry::new((**db).clone());
                let mut installed = 0;
                for skill in crate::skills::loader::builtin_skills() {
                    if !registry.is_installed(&skill.manifest.name).await {
                        match registry.install(&skill).await {
                            Ok(_) => {
                                installed += 1;
                                app.status_message = format!(
                                    "Installed '{}' v{}",
                                    skill.manifest.name, skill.manifest.version
                                );
                            }
                            Err(e) => {
                                app.status_message = format!("Install failed: {}", e);
                                break;
                            }
                        }
                    } else {
                        app.status_message =
                            format!("'{}' is already installed.", skill.manifest.name);
                    }
                }
                // Refresh installed list
                let registry = crate::skills::registry::SkillRegistry::new((**db).clone());
                app.installed_skills = registry.list_installed().await;
                if installed > 0 {
                    app.status_message = format!("Installed {} skill(s) successfully.", installed);
                }
            } else {
                app.status_message = "Database not available in remote mode.".to_string();
            }
            app.activity.bump();
        }
        KeyCode::F(3) => {
            if let Some(ref db) = app.db {
                let registry = crate::skills::registry::SkillRegistry::new((**db).clone());
                app.installed_skills = registry.list_installed().await;
                app.status_message = format!(
                    "Skills refreshed ({} installed).",
                    app.installed_skills.len()
                );
            }
            app.activity.bump();
        }
        KeyCode::Esc => {
            app.active_tab = ActiveTab::Chat;
            app.activity.bump();
        }
        _ => {}
    }
    Ok(())
}

async fn handle_search_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.show_search = false;
            app.search_query.clear();
            app.search_results = None;
            app.status_message = "Search closed.".to_string();
        }
        KeyCode::Enter => {
            if !app.search_query.is_empty() {
                let q = app.search_query.clone();
                match app.client.search_messages(&q).await {
                    Ok(results) => {
                        app.search_results = Some(results);
                        app.status_message = "Search complete.".to_string();
                    }
                    Err(e) => {
                        app.status_message = format!("Search failed: {}", e);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            if !app.search_query.is_empty() {
                let q = app.search_query.clone();
                app.search_results = app.client.search_messages(&q).await.ok();
            } else {
                app.search_results = None;
            }
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            if app.search_query.len() >= 2 {
                let q = app.search_query.clone();
                app.search_results = app.client.search_messages(&q).await.ok();
            }
        }
        _ => {}
    }
    app.activity.bump();
    Ok(())
}

async fn refresh_current_tab(app: &mut App) {
    match app.active_tab {
        ActiveTab::Agents => {
            let _ = app.refresh_agents().await;
        }
        ActiveTab::Supervisors => {
            let _ = app.refresh_supervisors().await;
        }
        ActiveTab::Tools => {
            let _ = app.refresh_tools().await;
        }
        ActiveTab::Workflows => {
            let _ = app.refresh_workflows().await;
        }
        ActiveTab::Computer => {
            let _ = app.refresh_computer().await;
        }
        ActiveTab::Settings => {
            let _ = app.refresh_settings_map().await;
            app.refresh_models().await;
        }
        ActiveTab::MCP => {
            if app.mcp_connected {
                let url = app.mcp_server_url.clone();
                let mut client = crate::mcp::client::McpClient::new(&url);
                if let Ok(tools) = client.list_tools().await {
                    app.mcp_tools = tools;
                }
                if let Ok(resources) = client.list_resources().await {
                    app.mcp_resources = resources;
                }
            }
        }
        ActiveTab::Metrics => {
            let _ = app.refresh_metrics().await;
        }
        ActiveTab::Chat => {
            let _ = app.refresh_conversations().await;
        }
        _ => {}
    }
}

async fn handle_chat_key(app: &mut App, key: KeyEvent, is_orch: bool) -> anyhow::Result<()> {
    match key.code {
        KeyCode::PageUp => app.chat_scroll = app.chat_scroll.saturating_add(8),
        KeyCode::PageDown => app.chat_scroll = app.chat_scroll.saturating_sub(8),
        KeyCode::Tab => {
            if app.input.starts_with('/') {
                if let Some(f) = crate::engine::commands::get_suggestions(&app.input).first() {
                    app.input = f.to_string();
                    app.input_cursor = app.input.len();
                    app.activity.bump();
                }
            }
        }
        KeyCode::Up => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx > 0 {
                    app.chat_scroll = 0;
                    app.selected_conv_id = Some(app.conversations[idx - 1].id);
                    app.refresh_conversations().await?;
                }
            }
        }
        KeyCode::Down => {
            if let Some(idx) = app.selected_conv_idx() {
                if idx + 1 < app.conversations.len() {
                    app.chat_scroll = 0;
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
        KeyCode::Backspace
        | KeyCode::Delete
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::Char(_) => {
            if !is_orch {
                super::input_handler::app(&key, app);
            }
        }
        KeyCode::Esc => {
            if is_orch {
                app.cancel_current_orchestrate();
                app.is_orchestrating
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                app.status_message = "Cancelled.".to_string();
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
        let s = crate::engine::commands::get_suggestions(&app.input);
        app.status_message = if s.is_empty() {
            "No matching command".into()
        } else {
            format!("Suggestions: {} (TAB to autocomplete)", s.join(", "))
        };
    }
    Ok(())
}
