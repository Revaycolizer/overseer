use super::app::App;
use super::ui::{ACCENT, MUTED, SUCCESS, SURFACE, TEXT, WARN};
use crate::engine::utils::discovery::AgentInfo;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn draw_agents(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]).split(area);

    draw_agent_list(f, app, chunks[0]);
    draw_agent_detail(f, app, chunks[1]);
}

fn draw_agent_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .agents
        .iter()
        .map(|a| {
            let prefix = if app.selected_agent_id == Some(a.id) {
                " ▶ "
            } else {
                "   "
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::new().fg(ACCENT)),
                Span::styled(a.name.clone(), Style::new().fg(TEXT)),
                Span::raw(" "),
                Span::styled(format!("[{}]", a.domain), Style::new().fg(MUTED)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SURFACE))
                .title(Span::styled(
                    " Agents ",
                    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .highlight_style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD));

    f.render_stateful_widget(list, area, &mut ListState::default());
}

fn draw_agent_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(agent) = app
        .agents
        .iter()
        .find(|a| app.selected_agent_id == Some(a.id))
    else {
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" ◀ ", Style::new().fg(ACCENT)),
                Span::styled("Select an agent from the list", Style::new().fg(MUTED)),
            ]))
            .block(block_detail("Agent Detail")),
            area,
        );
        return;
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("ID:       ", Style::new().fg(MUTED)),
            Span::styled(agent.id.to_string(), Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Name:     ", Style::new().fg(MUTED)),
            Span::styled(&agent.name, Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Domain:   ", Style::new().fg(MUTED)),
            Span::styled(&agent.domain, Style::new().fg(ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("Model:    ", Style::new().fg(MUTED)),
            Span::styled(&agent.model, Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Version:  ", Style::new().fg(MUTED)),
            Span::styled(&agent.version, Style::new().fg(TEXT)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled(
            "Description:",
            Style::new().fg(MUTED).add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(&agent.description, Style::new().fg(TEXT))),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled(
            "Tools:",
            Style::new().fg(MUTED).add_modifier(Modifier::BOLD),
        )]),
    ];

    let mut lines = lines;
    if agent.tools.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", Style::new().fg(MUTED))));
    } else {
        for tool in &agent.tools {
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::new().fg(ACCENT)),
                Span::styled(tool, Style::new().fg(TEXT)),
            ]));
        }
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![
        Span::styled("[F2] Edit  ", Style::new().fg(ACCENT)),
        Span::styled("[Del] Delete  ", Style::new().fg(WARN)),
        Span::styled("[F3] New Agent", Style::new().fg(SUCCESS)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(block_detail(&agent.name))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn block_detail(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(SURFACE))
        .title(Span::styled(
            format!(" {} ", title),
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
}
