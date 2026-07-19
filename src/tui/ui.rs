use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use super::app::{ActiveTab, App};
use super::chat_panel;
use super::info_panels;

pub const ACCENT: Color = Color::Rgb(59, 130, 246);
pub const SURFACE: Color = Color::Rgb(30, 41, 59);
pub const TEXT: Color = Color::Rgb(226, 232, 240);
pub const MUTED: Color = Color::Rgb(148, 163, 184);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARN: Color = Color::Rgb(250, 204, 21);

pub fn block(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(ACCENT).add_modifier(Modifier::DIM))
        .title(Span::styled(title, Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)))
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Length(3),  // tab bar
        Constraint::Min(0),     // main content
        Constraint::Length(3),  // status bar
    ])
    .split(f.size());

    // ── Header ──
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Overseer ", Style::new().fg(Color::White).bg(ACCENT).add_modifier(Modifier::BOLD)),
            Span::styled(" v0.1.4 ", Style::new().fg(MUTED)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))),
        chunks[0],
    );

    // ── Tab bar ──
    let tab_titles: Vec<Line> = ActiveTab::ALL.iter().map(|t| {
        let selected = t == &app.active_tab;
        let prefix = if selected { " ● " } else { "   " };
        let style = if selected {
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(MUTED)
        };
        Line::from(Span::styled(format!("{}{}", prefix, t.label()), style))
    }).collect();

    f.render_widget(
        Tabs::new(tab_titles)
            .select(app.active_tab.index())
            .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE)))
            .style(Style::new().fg(MUTED))
            .highlight_style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))
            .divider(Span::styled(" │ ", Style::new().fg(SURFACE))),
        chunks[1],
    );

    // ── Main content ──
    match app.active_tab {
        ActiveTab::Chat => chat_panel::draw_chat(f, app, chunks[2]),
        ActiveTab::Metrics => info_panels::draw_metrics(f, app, chunks[2]),
        ActiveTab::Browser => info_panels::draw_browser(f, app, chunks[2]),
        ActiveTab::SearchResults => draw_placeholder(f, "Search Results (Ctrl+F to search)", chunks[2]),
        ActiveTab::Agents => draw_placeholder(f, "Agent Manager — coming in Phase 2", chunks[2]),
        ActiveTab::Supervisors => draw_placeholder(f, "Supervisor Workshop — coming in Phase 2", chunks[2]),
        ActiveTab::Tools => draw_placeholder(f, "Tool Workshop — coming in Phase 2", chunks[2]),
        ActiveTab::Workflows => draw_placeholder(f, "Workflow Automation — coming in Phase 2", chunks[2]),
        ActiveTab::Computer => draw_placeholder(f, "Computer Control — coming in Phase 2", chunks[2]),
        ActiveTab::Settings => draw_placeholder(f, "Settings — coming in Phase 2", chunks[2]),
        ActiveTab::MCP => draw_placeholder(f, "MCP Protocol — coming in Phase 3", chunks[2]),
        ActiveTab::Skills => draw_placeholder(f, "Skill System — coming in Phase 4", chunks[2]),
    }

    // ── Status bar ──
    let status_text = if app.is_orchestrating.load(std::sync::atomic::Ordering::SeqCst) {
        format!(" ◆ {}  │  [Esc] Cancel  │  [Q]uit", app.status_message)
    } else {
        format!(
            " ◆ {}  │  [Tab/Arrows] Nav  │  [F1] Help  │  [F5] Refresh  │  [Q]uit",
            app.status_message
        )
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ ", Style::new().fg(ACCENT)),
            Span::styled(&app.status_message, Style::new().fg(TEXT)),
            Span::raw("  │  "),
            Span::styled("[Tab]", Style::new().fg(ACCENT)),
            Span::styled("/Arrows Nav  ", Style::new().fg(MUTED)),
            Span::styled("[F1]", Style::new().fg(ACCENT)),
            Span::styled(" Help  ", Style::new().fg(MUTED)),
            Span::styled("[F5]", Style::new().fg(ACCENT)),
            Span::styled(" Refresh  ", Style::new().fg(MUTED)),
            Span::styled("[Q]", Style::new().fg(ACCENT)),
            Span::styled("uit", Style::new().fg(MUTED)),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(Style::new().fg(SURFACE))),
        chunks[3],
    );
}

fn draw_placeholder(f: &mut Frame, title: &str, area: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ⏳ ", Style::new().fg(ACCENT)),
            Span::styled(title, Style::new().fg(MUTED)),
        ]))
        .block(block(""))
        .style(Style::new().fg(MUTED)),
        area,
    );
}
