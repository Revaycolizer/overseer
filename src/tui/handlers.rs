use crossterm::event::KeyCode;
use super::app::{ActiveTab, App};

pub async fn handle_key(app: &mut App, key: KeyCode) -> anyhow::Result<()> {
    match key {
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
            if app.active_tab == ActiveTab::Chat {
                app.send_message().await?;
            }
        }
        KeyCode::Backspace => {
            if app.active_tab == ActiveTab::Chat {
                app.input.pop();
            }
        }
        KeyCode::Char(c) => {
            if app.active_tab == ActiveTab::Chat {
                app.input.push(c);
            }
        }
        _ => {}
    }
    Ok(())
}
