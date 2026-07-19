use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SURFACE, SUCCESS, TEXT, WARN};

pub fn draw_supervisors(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(area);
    draw_supervisor_list(f, app, chunks[0]);
    draw_supervisor_detail(f, app, chunks[1]);
}

fn draw_supervisor_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .supervisors
        .iter()
        .map(|s| {
            let prefix = if app.selected_supervisor_id == Some(s.id) { " ▶ " } else { "   " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::new().fg(ACCENT)),
                Span::styled(s.name.clone(), Style::new().fg(TEXT)),
                Span::raw(" "),
                Span::styled(format!("[{}]", s.domain), Style::new().fg(MUTED)),
            ]))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(SURFACE))
                    .title(Span::styled(" Supervisors ", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))),
            )
            .highlight_style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
        area,
        &mut Default::default(),
    );
}

fn draw_supervisor_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(sup) = app.supervisors.iter().find(|s| app.selected_supervisor_id == Some(s.id)) else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" ◀  Select a supervisor", Style::new().fg(MUTED))))
                .block(detail_block("Supervisor Detail")),
            area,
        );
        return;
    };

    let model_display = sup.model.as_deref().unwrap_or("(default)");
    let desc_display = sup.description.as_deref().unwrap_or("(no description)");

    let mut lines = vec![
        Line::from(vec![Span::styled("ID:       ", Style::new().fg(MUTED)), Span::styled(sup.id.to_string(), Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Name:     ", Style::new().fg(MUTED)), Span::styled(&sup.name, Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Domain:   ", Style::new().fg(MUTED)), Span::styled(&sup.domain, Style::new().fg(ACCENT))]),
        Line::from(vec![Span::styled("Model:    ", Style::new().fg(MUTED)), Span::styled(model_display, Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Version:  ", Style::new().fg(MUTED)), Span::styled(sup.version.as_deref().unwrap_or("1.0"), Style::new().fg(TEXT))]),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled("Description:", Style::new().fg(MUTED).add_modifier(Modifier::BOLD))]),
        Line::from(Span::styled(desc_display, Style::new().fg(TEXT))),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled("Response format: ", Style::new().fg(MUTED)), Span::styled(&sup.response_format, Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Hide evidence: ", Style::new().fg(MUTED)), Span::styled(if sup.hide_evidence { "yes" } else { "no" }, Style::new().fg(TEXT))]),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled("[F2] Edit  ", Style::new().fg(ACCENT)), Span::styled("[Del] Delete  ", Style::new().fg(WARN)), Span::styled("[F3] New", Style::new().fg(SUCCESS))]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(detail_block(&sup.name)).wrap(Wrap { trim: false }),
        area,
    );
}

fn detail_block(title: &str) -> Block {
    Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
        .title(Span::styled(format!(" {} ", title), Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)))
}
