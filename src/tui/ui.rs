use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use super::app::{ActiveTab, App};
use super::chat_panel;
use super::info_panels;

pub const ACCENT: Color = Color::Rgb(59, 130, 246);
pub const SURFACE: Color = Color::Rgb(30, 41, 59);
pub const TEXT: Color = Color::Rgb(226, 232, 240);
pub const MUTED: Color = Color::Rgb(148, 163, 184);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARN: Color = Color::Rgb(250, 204, 21);

pub fn block(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(ACCENT).add_modifier(Modifier::DIM))
        .title(Span::styled(title, Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());

    let mode_label = match app.active_tab {
        ActiveTab::Chat => " Chat ",
        ActiveTab::Metrics => " Metrics ",
        ActiveTab::Browser => " Browser ",
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Overseer ", Style::new().fg(Color::White).bg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" v0.1.4 ", Style::new().fg(MUTED)),
            Span::raw(" │ "),
            Span::styled(mode_label, Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))),
        chunks[0],
    );

    match app.active_tab {
        ActiveTab::Chat => chat_panel::draw_chat(f, app, chunks[1]),
        ActiveTab::Metrics => info_panels::draw_metrics(f, app, chunks[1]),
        ActiveTab::Browser => info_panels::draw_browser(f, app, chunks[1]),
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ ", Style::new().fg(ACCENT)),
            Span::styled(&app.status_message, Style::new().fg(TEXT)),
            Span::raw("  │  "),
            Span::styled("[Q]uit", Style::new().fg(MUTED)),
            Span::raw("  "),
            Span::styled("[Tab]", Style::new().fg(ACCENT)),
            Span::styled(" Switch ", Style::new().fg(MUTED)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))),
        chunks[2],
    );
}
