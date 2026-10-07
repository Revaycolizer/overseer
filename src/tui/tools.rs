use super::app::App;
use super::ui::{ACCENT, MUTED, SUCCESS, SURFACE, TEXT, WARN};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw_tools(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::horizontal([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]).split(area);
    draw_tool_list(f, app, chunks[0]);
    draw_tool_detail(f, app, chunks[1]);
}

fn draw_tool_list(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .tools
        .iter()
        .map(|t| {
            let prefix = if app.selected_tool_id == Some(t.id) {
                " ▶ "
            } else {
                "   "
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::new().fg(ACCENT)),
                Span::styled(t.name.clone(), Style::new().fg(TEXT)),
                Span::styled(format!(" [{}]", t.language), Style::new().fg(MUTED)),
            ]))
        })
        .collect();

    f.render_stateful_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SURFACE))
                .title(Span::styled(
                    " Tools ",
                    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        ),
        area,
        &mut Default::default(),
    );
}

fn draw_tool_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(tool) = app
        .tools
        .iter()
        .find(|t| app.selected_tool_id == Some(t.id))
    else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " ◀  Select a tool",
                Style::new().fg(MUTED),
            )))
            .block(detail_block("Tool Detail")),
            area,
        );
        return;
    };

    let safety = &tool.safety_level;
    let mut lines = vec![
        Line::from(vec![
            Span::styled("ID:       ", Style::new().fg(MUTED)),
            Span::styled(tool.id.to_string(), Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Name:     ", Style::new().fg(MUTED)),
            Span::styled(&tool.name, Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Language: ", Style::new().fg(MUTED)),
            Span::styled(&tool.language, Style::new().fg(ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("Safety:   ", Style::new().fg(MUTED)),
            Span::styled(
                safety,
                Style::new().fg(match safety.as_str() {
                    "dangerous" => WARN,
                    _ => TEXT,
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Version:  ", Style::new().fg(MUTED)),
            Span::styled(&tool.version, Style::new().fg(TEXT)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![Span::styled(
            "Description:",
            Style::new().fg(MUTED).add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(&tool.description, Style::new().fg(TEXT))),
    ];

    if let Some(intent) = &tool.required_intent {
        lines.push(Line::from(Span::raw("")));
        lines.push(Line::from(vec![
            Span::styled("Required intent: ", Style::new().fg(MUTED)),
            Span::styled(intent, Style::new().fg(TEXT)),
        ]));
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![
        Span::styled("[F2] Edit  ", Style::new().fg(ACCENT)),
        Span::styled("[F3] Run Test  ", Style::new().fg(SUCCESS)),
    ]));

    f.render_widget(
        Paragraph::new(lines)
            .block(detail_block(&tool.name))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn detail_block(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(SURFACE))
        .title(Span::styled(
            format!(" {} ", title),
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
}
