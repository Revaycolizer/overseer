use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use super::app::App;
use super::ui::{ACCENT, MUTED, SURFACE, TEXT};

/// Draw a universal search overlay.
///
/// The search bar is rendered as a centred overlay. Results are shown below
/// as the user types. Press Esc to close, Enter to select a result.
pub fn draw_search(f: &mut Frame, app: &mut App, area: Rect) {
    let overlay = Rect {
        x: area.width / 6,
        y: area.height / 4,
        width: area.width * 2 / 3,
        height: area.height / 3,
    };

    f.render_widget(Clear, overlay);

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" 🔍 ", Style::new().fg(ACCENT)),
            Span::styled("Universal Search", Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled("  — search conversations, agents, tools", Style::new().fg(MUTED)),
        ]),
        Line::from(Span::raw("")),
    ];

    // Search input
    let display = if app.search_query.is_empty() {
        " Type to search...".to_string()
    } else {
        format!(" {}", app.search_query)
    };
    lines.push(Line::from(Span::styled(display, Style::new().fg(TEXT))));
    lines.push(Line::from(Span::raw("")));

    // Results
    if let Some(ref results) = app.search_results {
        if results.is_empty() {
            lines.push(Line::from(Span::styled("  No results found.", Style::new().fg(MUTED))));
        } else {
            for (i, r) in results.iter().enumerate().take(10) {
                let snippet: String = r.content.chars().take(80).collect();
                lines.push(Line::from(vec![
                    Span::styled(format!("  {} ", i + 1), Style::new().fg(ACCENT)),
                    Span::styled(snippet, Style::new().fg(TEXT)),
                    Span::styled(format!(" [{}]", r.conversation_title), Style::new().fg(MUTED)),
                ]));
            }
            if results.len() > 10 {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", results.len() - 10),
                    Style::new().fg(MUTED),
                )));
            }
        }
    } else if !app.search_query.is_empty() {
        lines.push(Line::from(Span::styled("  Searching...", Style::new().fg(MUTED))));
    } else {
        lines.push(Line::from(Span::styled(
            "  Search across conversations, agents, tools, and supervisors.",
            Style::new().fg(MUTED),
        )));
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(Span::styled(
        " Enter=select  Esc=close  Type to search",
        Style::new().fg(MUTED),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(ACCENT))
                    .style(Style::new().bg(SURFACE)),
            )
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false }),
        overlay,
    );
}
