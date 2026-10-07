use super::app::{ActiveTab, App};
use super::{
    agents, chat_panel, computer, help as help_panel, info_panels, mcp, search as search_panel,
    settings, skills, supervisors, tools, workflows,
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
    Frame,
};

pub const BACKGROUND: Color = Color::Rgb(28, 30, 27);
pub const ACCENT: Color = Color::Rgb(225, 218, 195);
pub const SURFACE: Color = Color::Rgb(76, 78, 70);
pub const TEXT: Color = Color::Rgb(243, 241, 232);
pub const MUTED: Color = Color::Rgb(176, 179, 164);
pub const SUCCESS: Color = Color::Rgb(34, 197, 94);
pub const WARN: Color = Color::Rgb(250, 204, 21);

pub fn block(title: &str) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(ACCENT).add_modifier(Modifier::DIM))
        .title(Span::styled(
            title,
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
}

pub fn draw(f: &mut Frame, app: &mut App) {
    f.render_widget(
        Block::default().style(Style::new().fg(TEXT).bg(BACKGROUND)),
        f.area(),
    );
    let chunks = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(3), // tab bar
        Constraint::Min(0),    // main content
        Constraint::Length(3), // status bar
    ])
    .split(f.area());

    // Header
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " Overseer ",
                Style::new()
                    .fg(Color::Rgb(41, 43, 40))
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  F6 Models  |  Tab Next  |  Ctrl+Q Quit",
                Style::new().fg(MUTED),
            ),
        ])),
        chunks[0],
    );

    // Tab bar
    let count = (f.area().width as usize / 18)
        .max(1)
        .min(ActiveTab::ALL.len());
    let start = app
        .active_tab
        .index()
        .saturating_sub(count / 2)
        .min(ActiveTab::ALL.len() - count);
    let tab_titles: Vec<Line> = ActiveTab::ALL[start..start + count]
        .iter()
        .map(|t| {
            let selected = t == &app.active_tab;
            let prefix = if selected { " ● " } else { "   " };
            let style = if selected {
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(MUTED)
            };
            Line::from(Span::styled(format!("{}{}", prefix, t.label()), style))
        })
        .collect();

    f.render_widget(
        Tabs::new(tab_titles)
            .select(app.active_tab.index().saturating_sub(start))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::new().fg(SURFACE)),
            )
            .style(Style::new().fg(MUTED))
            .highlight_style(Style::new().fg(ACCENT).add_modifier(Modifier::BOLD))
            .divider(Span::styled(" │ ", Style::new().fg(SURFACE))),
        chunks[1],
    );

    // Main content
    match app.active_tab {
        ActiveTab::Chat => chat_panel::draw_chat(f, app, chunks[2]),
        ActiveTab::Metrics => info_panels::draw_metrics(f, app, chunks[2]),
        ActiveTab::Browser => info_panels::draw_browser(f, app, chunks[2]),
        ActiveTab::SearchResults => {
            draw_placeholder(f, "Search Results (Ctrl+F to search)", chunks[2])
        }
        ActiveTab::Agents => agents::draw_agents(f, app, chunks[2]),
        ActiveTab::Supervisors => supervisors::draw_supervisors(f, app, chunks[2]),
        ActiveTab::Tools => tools::draw_tools(f, app, chunks[2]),
        ActiveTab::Workflows => workflows::draw_workflows(f, app, chunks[2]),
        ActiveTab::Computer => computer::draw_computer(f, app, chunks[2]),
        ActiveTab::Settings => settings::draw_settings(f, app, chunks[2]),
        ActiveTab::MCP => mcp::draw_mcp(f, app, chunks[2]),
        ActiveTab::Skills => skills::draw_skills(f, app, chunks[2]),
    }

    // Status bar
    let status_text = if app
        .is_orchestrating
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        format!(
            " ◆ {}  │  [Esc] Cancel  │  [Ctrl+Q] Quit",
            app.status_message
        )
    } else {
        app.status_message.clone()
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ◆ ", Style::new().fg(ACCENT)),
            Span::styled(status_text, Style::new().fg(TEXT)),
            Span::raw("  │  "),
            Span::styled("[Tab/Shift+Tab]", Style::new().fg(ACCENT)),
            Span::styled(" Tabs  ", Style::new().fg(MUTED)),
            Span::styled("[F1]", Style::new().fg(ACCENT)),
            Span::styled(" Help  ", Style::new().fg(MUTED)),
            Span::styled("[Ctrl+Q]", Style::new().fg(ACCENT)),
            Span::styled(" Quit", Style::new().fg(MUTED)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(SURFACE)),
        ),
        chunks[3],
    );

    // Overlays (drawn last for z-order)
    if app.show_help {
        help_panel::draw_help(f, app, f.area());
    }
    if app.show_search {
        search_panel::draw_search(f, app, f.area());
    }
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
