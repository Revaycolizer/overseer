use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use super::app::{ActiveTab, App};

pub async fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let is_orch = app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst);

    if key.code == KeyCode::F(2) || (key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL)) {
        let new_id = app.client.add_conversation("New Conversation").await?;
        app.activity.bump();
        app.refresh_conversations().await?;
        if let Some(idx) = app.conversations.iter().position(|c| c.id == new_id) {
            app.selected_conv_idx = idx;
            let _ = app.refresh_conversations().await;
        }
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
            if app.active_tab == ActiveTab::Chat && app.selected_conv_idx > 0 {
                app.selected_conv_idx -= 1;
                app.refresh_conversations().await?;
            }
        }
        KeyCode::Down => {
            if app.active_tab == ActiveTab::Chat && app.selected_conv_idx + 1 < app.conversations.len() {
                app.selected_conv_idx += 1;
                app.refresh_conversations().await?;
            }
        }
        KeyCode::Enter => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                app.send_message().await?;
            }
        }
        KeyCode::Backspace => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                app.input.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.active_tab == ActiveTab::Chat && !is_orch {
                app.input.push(c);
            }
        }
        _ => {}
    }
    Ok(())
}
