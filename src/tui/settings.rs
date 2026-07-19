use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SURFACE, TEXT};

pub fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(area);

    draw_setting_list(f, app, chunks[0]);
    draw_setting_detail(f, app, chunks[1]);
}

fn draw_setting_list(f: &mut Frame, app: &mut App, area: Rect) {
    let mut settings: Vec<(&str, &str)> = vec![
        ("default_chat_model", "Default LLM model for chat"),
        ("planner_model", "Model used by the planner"),
        ("title_generator_model", "Model for conversation titles"),
        ("max_iterations", "Max agent iterations per request"),
        ("tool_timeout_ms", "Tool execution timeout (ms)"),
        ("model_timeout_ms", "LLM response timeout (ms)"),
        ("lattice_enabled", "Enable V3 lattice architecture"),
        ("lattice_max_concurrency", "Max parallel cells"),
        ("redis_url", "Redis cache URL (optional)"),
    ];

    let items: Vec<ListItem> = settings.iter().map(|(key, desc)| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("   {} ", key), Style::new().fg(TEXT)),
            Span::styled(format!("({})", desc), Style::new().fg(MUTED)),
        ]))
    }).collect();

    f.render_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(SURFACE))
                    .title(Span::styled(" Settings ", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))),
            ),
        area,
    );
}

fn draw_setting_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Settings Management", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Use ", Style::new().fg(TEXT)),
            Span::styled("/model <name>", Style::new().fg(ACCENT)),
            Span::styled(" to change the chat model.", Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Use the ", Style::new().fg(TEXT)),
            Span::styled("/settings", Style::new().fg(ACCENT)),
            Span::styled(" command to return here.", Style::new().fg(TEXT)),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Full settings editing with inline value", Style::new().fg(MUTED))),
        Line::from(Span::styled("modification is coming in a future phase.", Style::new().fg(MUTED))),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(SURFACE))
                    .title(Span::styled(" Detail ", Style::new().fg(ACCENT))),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}
