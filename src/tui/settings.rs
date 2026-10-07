use super::app::App;
use super::ui::{self, ACCENT, MUTED, SURFACE, TEXT};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).split(area);
    let active = app
        .settings_map
        .get("default_chat_model")
        .filter(|m| m.as_str() != "__use_first_available__")
        .cloned()
        .unwrap_or_else(|| "Automatic (first installed model)".into());
    f.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Chat model: {active}")),
            Line::from("↑/↓ choose · Enter apply · F5 refresh"),
            Line::from("Uses Ollama for chat + planner. Esc: back."),
        ])
        .style(Style::new().fg(TEXT))
        .wrap(Wrap { trim: true })
        .block(ui::block(" Ollama models ")),
        chunks[0],
    );

    if let Some(error) = &app.model_error {
        f.render_widget(
            Paragraph::new(format!("{error}\nStart Ollama, then press F5 to retry."))
                .style(Style::new().fg(MUTED))
                .wrap(Wrap { trim: true })
                .block(ui::block(" Connection ")),
            chunks[1],
        );
    } else if app.available_models.is_empty() {
        f.render_widget(
            Paragraph::new("No models installed. Pull a model in Ollama, then press F5.")
                .style(Style::new().fg(MUTED))
                .wrap(Wrap { trim: true })
                .block(ui::block(" Installed models ")),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .available_models
            .iter()
            .map(|m| {
                let current = if m == &active { "  [active]" } else { "" };
                ListItem::new(format!(" {m}{current}")).style(Style::new().fg(TEXT))
            })
            .collect();
        let mut state = ListState::default();
        state.select(Some(app.selected_model));
        f.render_stateful_widget(
            List::new(items)
                .block(ui::block(" Installed models "))
                .highlight_symbol("> ")
                .highlight_style(
                    Style::new()
                        .fg(ACCENT)
                        .bg(SURFACE)
                        .add_modifier(Modifier::BOLD),
                ),
            chunks[1],
            &mut state,
        );
    }
}
