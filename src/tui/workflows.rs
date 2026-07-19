use crate::backend::workflows::models::Workflow;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SURFACE, SUCCESS, TEXT, WARN};

pub fn draw_workflows(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .split(area);
    draw_workflow_list(f, app, chunks[0]);
    draw_workflow_detail(f, app, chunks[1]);
}

fn draw_workflow_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .workflows
        .iter()
        .map(|w| {
            let prefix = if app.selected_workflow_id == Some(w.id) { " ▶ " } else { "   " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::new().fg(ACCENT)),
                Span::styled(w.name.clone(), Style::new().fg(TEXT)),
                Span::styled(format!(" [{}]", w.status), Style::new().fg(match w.status.as_str() {
                    "active" => SUCCESS, "failed" => WARN, _ => MUTED
                })),
            ]))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Workflows ", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)))),
        area,
        &mut Default::default(),
    );
}

fn draw_workflow_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(wf) = app.workflows.iter().find(|w| app.selected_workflow_id == Some(w.id)) else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(" ◀  Select a workflow", Style::new().fg(MUTED))))
                .block(detail_block("Workflow Detail")),
            area,
        );
        return;
    };

    let desc = wf.description.as_deref().unwrap_or("(no description)");
    let trigger = &wf.trigger_type;
    let mut lines = vec![
        Line::from(vec![Span::styled("ID:      ", Style::new().fg(MUTED)), Span::styled(wf.id.to_string(), Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Name:    ", Style::new().fg(MUTED)), Span::styled(&wf.name, Style::new().fg(TEXT))]),
        Line::from(vec![Span::styled("Status:  ", Style::new().fg(MUTED)), Span::styled(&wf.status, Style::new().fg(match wf.status.as_str() {
            "active" => SUCCESS, "failed" => WARN, _ => TEXT
        }))]),
        Line::from(vec![Span::styled("Trigger: ", Style::new().fg(MUTED)), Span::styled(trigger, Style::new().fg(ACCENT))]),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled("Description:", Style::new().fg(MUTED).add_modifier(Modifier::BOLD))]),
        Line::from(Span::styled(desc, Style::new().fg(TEXT))),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Step-level view coming when workflow_steps are loaded.", Style::new().fg(MUTED))),
    ];

    f.render_widget(
        Paragraph::new(lines).block(detail_block(&wf.name)).wrap(Wrap { trim: false }),
        area,
    );
}

fn detail_block(title: &str) -> Block {
    Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
        .title(Span::styled(format!(" {} ", title), Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)))
}
