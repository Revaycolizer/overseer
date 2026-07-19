use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SUCCESS, SURFACE, TEXT, WARN};

pub fn draw_skills(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    draw_skills_header(f, app, chunks[0]);
    draw_skills_list(f, app, chunks[1]);
}

fn draw_skills_header(f: &mut Frame, app: &mut App, area: Rect) {
    let builtin_count = 1usize;
    let installed_count = app.installed_skills.len();

    let lines = vec![
        Line::from(vec![
            Span::styled("Skill System", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(format!("{} installed", installed_count), Style::new().fg(SUCCESS)),
            Span::raw("  │  "),
            Span::styled(format!("{} available", builtin_count), Style::new().fg(MUTED)),
        ]),
        Line::from(Span::raw("")),
        Line::from(Span::styled("Skills are reusable bundles of agents, supervisors, and tools.", Style::new().fg(TEXT))),
        Line::from(vec![
            Span::styled("[F2] Install Web Researcher  ", Style::new().fg(SUCCESS)),
            Span::styled("[F3] Refresh  ", Style::new().fg(ACCENT)),
            Span::styled("[Del] Uninstall  ", Style::new().fg(WARN)),
            Span::styled("[Esc] Back", Style::new().fg(MUTED)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Skills ", Style::new().fg(ACCENT)))),
        area,
    );
}

fn draw_skills_list(f: &mut Frame, app: &mut App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    if !app.installed_skills.is_empty() {
        lines.push(Line::from(Span::styled("Installed:", Style::new().fg(SUCCESS).add_modifier(Modifier::BOLD))));
        for skill in &app.installed_skills {
            lines.push(Line::from(vec![
                Span::styled("  ✓ ", Style::new().fg(SUCCESS)),
                Span::styled(&skill.name, Style::new().fg(TEXT)),
                Span::styled(format!(" v{}", skill.version), Style::new().fg(ACCENT)),
                Span::raw(" — "),
                Span::styled(&skill.description, Style::new().fg(MUTED)),
            ]));
        }
        lines.push(Line::from(Span::raw("")));
    }

    // Show built-in skills as available
    lines.push(Line::from(Span::styled("Available:", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))));
    let skill_names = ["web-researcher (v1.0.0) — Research any topic using web search and synthesis"];
    for name in &skill_names {
        let installed = app.installed_skills.iter().any(|s| name.starts_with(&s.name));
        if !installed {
            lines.push(Line::from(vec![
                Span::styled("  + ", Style::new().fg(ACCENT)),
                Span::styled(*name, Style::new().fg(TEXT)),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled("  No skills available.", Style::new().fg(MUTED))));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Registry ", Style::new().fg(ACCENT))))
            .wrap(Wrap { trim: false }),
        area,
    );
}
