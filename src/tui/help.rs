use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use super::app::{ActiveTab, App};
use super::ui::{ACCENT, MUTED, SURFACE, TEXT};

pub fn draw_help(f: &mut Frame, app: &mut App, area: Rect) {
    let overlay = Rect {
        x: area.width / 5,
        y: area.height / 6,
        width: area.width * 3 / 5,
        height: area.height * 2 / 3,
    };

    f.render_widget(Clear, overlay);

    let tab_name = app.active_tab.label();

    let (header, items): (&str, Vec<(&str, bool)>) = match app.active_tab {
        ActiveTab::Chat => ("Chat", vec![
            ("Ctrl+N / F2    New conversation", false),
            ("Enter           Send message", false),
            ("Up/Down         Select conversation", false),
            ("Esc             Cancel / clear search", false),
            ("", false),
            ("Slash: /new, /search, /model, /export, /regen,", false),
            ("       /rename, /settings, /agents, /supervisors, /tools", false),
        ]),
        ActiveTab::Computer => ("Computer Control", vec![
            ("F2              Activate session", false),
            ("F3              Deactivate session", false),
            ("F4              Approve pending action", false),
            ("F5              Reject pending action", false),
            ("F6              Emergency stop", false),
            ("Esc             Return to Chat", false),
        ]),
        ActiveTab::MCP => ("MCP Protocol", vec![
            ("F2              Connect to MCP server", false),
            ("F3              Disconnect", false),
            ("F5              Refresh tools/resources", false),
            ("Esc             Return to Chat", false),
        ]),
        _ => (tab_name, vec![
            ("Up/Down         Select item", false),
            ("Esc             Return to Chat", false),
            ("F5              Refresh", false),
        ]),
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" Help — {} ", header),
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::raw("")));

    for (text, _is_header) in &items {
        if text.is_empty() {
            lines.push(Line::from(Span::raw("")));
        } else {
            lines.push(Line::from(Span::styled(*text, Style::new().fg(TEXT))));
        }
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " Global: Ctrl+Left/Right=tabs  Alt+1..0=jump  F5=Refresh  Q=Quit  F1=Help",
        Style::new().fg(MUTED),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(ACCENT)))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        overlay,
    );
}
