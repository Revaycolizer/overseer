use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use super::app::{ActiveTab, App};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)])
        .split(f.size());

    f.render_widget(Paragraph::new(Span::styled(
        format!("OVERSEER v0.1.4 | Mode: {}", match app.active_tab {
            ActiveTab::Chat => " [CHAT CONVERSATIONS] ",
            ActiveTab::Metrics => " [SYSTEM METRICS] ",
        }),
        Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
    )).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)).title(" Overseer Mission Control ")), chunks[0]);

    f.render_widget(Paragraph::new(Line::from(Span::styled(
        format!(" {} | Q/q: Quit | TAB: Switch View", app.status_message),
        Style::default().fg(Color::Yellow),
    ))).block(Block::default().borders(Borders::ALL)), chunks[2]);

    match app.active_tab {
        ActiveTab::Chat => draw_chat(f, app, chunks[1]),
        ActiveTab::Metrics => draw_metrics(f, app, chunks[1]),
    }
}

fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let chat_layout = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)]).split(area);

    let convs: Vec<ListItem> = app.conversations.iter().enumerate().map(|(i, c)| {
        let prefix = if i == app.selected_conv_idx { "▶ " } else { "  " };
        let style = if i == app.selected_conv_idx { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };
        let display_title = if c.title.is_empty() { format!("Chat {}", c.id) } else { c.title.clone() };
        ListItem::new(Line::from(vec![Span::styled(prefix, style), Span::styled(display_title, style)]))
    }).collect();

    f.render_widget(List::new(convs).block(Block::default().borders(Borders::ALL).title(" Chats (Use UP/DOWN) ")), chat_layout[0]);

    let right_layout = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)]).split(chat_layout[1]);

    let messages_text: Vec<Line> = app.messages.iter()
        .flat_map(super::message_view::format_message)
        .collect();

    let token_count = crate::engine::tokenizer::count_messages_tokens(&app.messages);
    f.render_widget(
        Paragraph::new(messages_text)
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Conversation History (~{} tokens) ", token_count
            )))
            .wrap(ratatui::widgets::Wrap { trim: true }),
        right_layout[0],
    );
    // Render the input with a visible cursor: split the string at
    // `input_cursor` and insert a "▌" so the user can see where the
    // next character will land and which character is to the right of
    // the cursor. Cursor is clamped on the TUI side via the input
    // handler (Left/Right/Home/End/Delete/Backspace).
    let (left, right) = app.input.split_at(app.input_cursor.min(app.input.len()));
    let input_line = Line::from(vec![
        Span::styled(left.to_string(), Style::default().fg(Color::White)),
        Span::styled("▌".to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled(right.to_string(), Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(input_line).block(Block::default().borders(Borders::ALL).title(" Type Message (←/→ to move cursor, ENTER to send) ")), right_layout[1]);
}

fn draw_metrics(f: &mut Frame, app: &App, area: Rect) {
    let mut text = vec![Line::from(Span::styled("SYSTEM STACK METRICS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))), Line::from("")];
    if let Some(load) = &app.system_load {
        text.push(Line::from(format!("System Metric: {}", load.label)));
        text.push(Line::from(format!("Usage Level:   {}%", load.usage)));
    } else {
        text.push(Line::from("Retrieving metrics..."));
    }
    f.render_widget(Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" System Status Overview ")).wrap(ratatui::widgets::Wrap { trim: true }), area);
}
