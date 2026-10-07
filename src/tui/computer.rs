use super::app::App;
use super::ui::{ACCENT, MUTED, SUCCESS, SURFACE, TEXT, WARN};
use crate::tui::app::ActionEntry;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn draw_computer(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(10), Constraint::Min(0)]).split(area);

    draw_session_status(f, app, chunks[0]);
    draw_action_log(f, app, chunks[1]);
}

fn draw_session_status(f: &mut Frame, app: &mut App, area: Rect) {
    let is_active = app
        .computer_state
        .as_ref()
        .map(|s| s.session_state == "active" || s.session_state == "running")
        .unwrap_or(false);
    let mode = app
        .computer_state
        .as_ref()
        .map(|s| s.supervision_mode.as_str())
        .unwrap_or("none");
    let pending = app
        .computer_state
        .as_ref()
        .map(|s| s.approval_state.as_str())
        .unwrap_or("none");

    let lines = vec![
        Line::from(vec![
            Span::styled(
                "Computer Control",
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            if is_active {
                Span::styled("● ACTIVE", Style::new().fg(SUCCESS))
            } else {
                Span::styled("○ INACTIVE", Style::new().fg(MUTED))
            },
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Session mode:    ", Style::new().fg(MUTED)),
            Span::styled(mode, Style::new().fg(ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("Approval state:  ", Style::new().fg(MUTED)),
            Span::styled(
                pending,
                Style::new().fg(match pending {
                    "pending" => WARN,
                    "approved" => SUCCESS,
                    _ => TEXT,
                }),
            ),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("[F2] Activate  ", Style::new().fg(SUCCESS)),
            Span::styled("[F3] Deactivate  ", Style::new().fg(WARN)),
            Span::styled("[F4] Approve  ", Style::new().fg(ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("[F5] Reject  ", Style::new().fg(MUTED)),
            Span::styled(
                "[F6] Emergency Stop  ",
                Style::new().fg(WARN).add_modifier(Modifier::BOLD),
            ),
            Span::styled("[Esc] Back to Chat", Style::new().fg(MUTED)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Session ", Style::new().fg(ACCENT))),
        ),
        area,
    );
}

fn draw_action_log(f: &mut Frame, app: &mut App, area: Rect) {
    let lines: Vec<Line> = if app.computer_actions.is_empty() {
        vec![Line::from(Span::styled(
            "  No actions logged yet.",
            Style::new().fg(MUTED),
        ))]
    } else {
        app.computer_actions
            .iter()
            .rev()
            .take(20)
            .map(|entry| {
                Line::from(vec![
                    Span::styled(
                        &entry.status,
                        Style::new().fg(match entry.status.as_str() {
                            "approved" => SUCCESS,
                            "rejected" | "failed" => WARN,
                            _ => TEXT,
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled(&entry.tool, Style::new().fg(ACCENT)),
                    Span::raw(" "),
                    Span::styled(&entry.detail, Style::new().fg(TEXT)),
                ])
            })
            .collect()
    };

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(SURFACE))
                    .title(Span::styled(" Action Log ", Style::new().fg(ACCENT))),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}
