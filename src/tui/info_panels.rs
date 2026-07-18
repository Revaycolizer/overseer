use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use super::app::App;
use super::ui;

pub fn draw_metrics(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(" System Overview", Style::new().fg(ui::WARN).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];
    if let Some(load) = &app.system_load {
        let usage = load.usage;
        let bar = "█".repeat((usage as usize).min(100) / 5);
        let color = if usage > 80 { Color::Red } else if usage > 50 { ui::WARN } else { ui::SUCCESS };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", load.label), Style::new().fg(ui::TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}% ", usage), Style::new().fg(color)),
            Span::styled(bar, Style::new().fg(color)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(" Retrieving metrics...", Style::new().fg(ui::MUTED))));
    }
    f.render_widget(Paragraph::new(lines).block(ui::block(" Metrics ")), area);
}

pub fn draw_browser(f: &mut Frame, _app: &App, area: Rect) {
    let lines = vec![
        Line::from(Span::styled(" Browser Sessions", Style::new().fg(ui::ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ◆ ", Style::new().fg(ui::SUCCESS)),
            Span::styled("Use the Dioxus Web UI to manage sessions.", Style::new().fg(ui::TEXT)),
        ]),
        Line::from(vec![
            Span::styled(" ◆ ", Style::new().fg(ui::ACCENT)),
            Span::styled("REST API:  /v1/browser/sessions", Style::new().fg(ui::MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled(" TUI management coming soon.", Style::new().fg(ui::MUTED).add_modifier(Modifier::ITALIC))),
    ];
    f.render_widget(Paragraph::new(lines).block(ui::block(" Browser ")), area);
}
