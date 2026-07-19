use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SUCCESS, SURFACE, TEXT, WARN};

pub fn draw_mcp(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(0)])
        .split(area);

    draw_mcp_status(f, app, chunks[0]);
    draw_mcp_detail(f, app, chunks[1]);
}

fn draw_mcp_status(f: &mut Frame, app: &mut App, area: Rect) {
    let connected = app.mcp_connected;
    let tools_count = app.mcp_tools.len();
    let resources_count = app.mcp_resources.len();

    let lines = vec![
        Line::from(vec![
            Span::styled("Model Context Protocol (MCP)", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            if connected {
                Span::styled("● CONNECTED", Style::new().fg(SUCCESS))
            } else {
                Span::styled("○ DISCONNECTED", Style::new().fg(MUTED))
            },
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("Server:    ", Style::new().fg(MUTED)),
            Span::styled(&app.mcp_server_url, Style::new().fg(TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Tools:     ", Style::new().fg(MUTED)),
            Span::styled(tools_count.to_string(), Style::new().fg(ACCENT)),
            Span::styled(" available", Style::new().fg(MUTED)),
        ]),
        Line::from(vec![
            Span::styled("Resources: ", Style::new().fg(MUTED)),
            Span::styled(resources_count.to_string(), Style::new().fg(ACCENT)),
            Span::styled(" available", Style::new().fg(MUTED)),
        ]),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled("[F2] Connect  ", Style::new().fg(SUCCESS)),
            Span::styled("[F3] Disconnect  ", Style::new().fg(WARN)),
            Span::styled("[F5] Refresh  ", Style::new().fg(ACCENT)),
            Span::styled("[Esc] Back", Style::new().fg(MUTED)),
        ]),
    ];

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Connection ", Style::new().fg(ACCENT)))),
        area,
    );
}

fn draw_mcp_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    if app.mcp_tools.is_empty() && app.mcp_resources.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No MCP server connected. Press F2 to connect to the local Overseer MCP server.",
            Style::new().fg(MUTED),
        )));
    } else {
        if !app.mcp_tools.is_empty() {
            lines.push(Line::from(Span::styled("Tools:", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))));
            for tool in &app.mcp_tools {
                lines.push(Line::from(vec![
                    Span::styled("  • ", Style::new().fg(SUCCESS)),
                    Span::styled(&tool.name, Style::new().fg(TEXT)),
                    Span::raw(" — "),
                    Span::styled(&tool.description, Style::new().fg(MUTED)),
                ]));
            }
            lines.push(Line::from(Span::raw("")));
        }

        if !app.mcp_resources.is_empty() {
            lines.push(Line::from(Span::styled("Resources:", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))));
            for res in &app.mcp_resources {
                lines.push(Line::from(vec![
                    Span::styled("  ◆ ", Style::new().fg(ACCENT)),
                    Span::styled(&res.name, Style::new().fg(TEXT)),
                    Span::raw(" "),
                    Span::styled(&res.uri, Style::new().fg(MUTED)),
                ]));
            }
        }
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))
                .title(Span::styled(" Discovery ", Style::new().fg(ACCENT))))
            .wrap(Wrap { trim: false }),
        area,
    );
}
