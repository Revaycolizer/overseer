use super::app::App;
use super::ui;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

pub fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    if area.width < 72 {
        let right = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);
        draw_messages(f, app, right[0]);
        draw_input(f, app, right[1]);
        return;
    }
    let layout = Layout::horizontal([Constraint::Length(26), Constraint::Min(0)]).split(area);
    draw_sidebar(f, app, layout[0]);
    let right = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(layout[1]);
    draw_messages(f, app, right[0]);
    draw_input(f, app, right[1]);
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let selected_idx = app.selected_conv_idx();
    let convs: Vec<ListItem> = app
        .conversations
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let is_sel = selected_idx == Some(i);
            let title = if c.title.is_empty() {
                format!("Chat {}", c.id)
            } else {
                c.title.clone()
            };
            let icon = if is_sel { "▸ " } else { "  " };
            let style = if is_sel {
                Style::new().fg(ui::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(ui::MUTED)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", icon, title),
                style,
            )))
        })
        .collect();
    f.render_widget(
        List::new(convs).block(ui::block(" Chats ").border_style(Style::new().fg(ui::SURFACE))),
        area,
    );
}

fn draw_messages(f: &mut Frame, app: &App, area: Rect) {
    let messages: Vec<Line> = if let Some(ref results) = app.search_results {
        let mut lines = vec![
            Line::from(Span::styled(
                " Search Results",
                Style::new().fg(ui::WARN).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(" ESC to exit", Style::new().fg(ui::MUTED))),
            Line::from(""),
        ];
        if results.is_empty() {
            lines.push(Line::from(Span::styled(
                " No matches",
                Style::new().fg(ui::MUTED),
            )));
        } else {
            for (idx, r) in results.iter().enumerate() {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}.", idx + 1), Style::new().fg(ui::ACCENT)),
                    Span::raw(" "),
                    Span::styled(
                        &r.conversation_title,
                        Style::new().fg(ui::SUCCESS).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        r.content.chars().take(60).collect::<String>(),
                        Style::new().fg(ui::TEXT),
                    ),
                ]));
            }
        }
        lines
    } else if app.messages.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                " What would you like to do?",
                Style::new().fg(ui::TEXT),
            )),
            Line::from(" Type a message and press Enter."),
            Line::from(" Ctrl+N: new chat · F1: help · Ctrl+Q: quit"),
        ]
    } else {
        app.messages
            .iter()
            .flat_map(super::message_view::format_message)
            .collect()
    };

    let title = if app.search_results.is_some() {
        format!(
            " Search Results ({}) ",
            app.search_results.as_ref().unwrap().len()
        )
    } else {
        let tokens = crate::engine::tokenizer::count_messages_tokens(&app.messages);
        format!(" Messages (~{} tokens) ", tokens)
    };

    let paragraph = Paragraph::new(messages)
        .block(ui::block(&title))
        .wrap(ratatui::widgets::Wrap { trim: true });
    let max_scroll = paragraph
        .line_count(area.width.saturating_sub(2))
        .saturating_sub(area.height.saturating_sub(2) as usize) as u16;
    f.render_widget(
        paragraph.scroll((max_scroll.saturating_sub(app.chat_scroll), 0)),
        area,
    );
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let model = app
        .settings_map
        .get("default_chat_model")
        .filter(|m| m.as_str() != "__use_first_available__")
        .map(String::as_str)
        .unwrap_or("Auto");
    let title = format!(" Message · {model} · F6 models ");
    let cursor = ratatui::text::Line::from(&app.input[..app.input_cursor.min(app.input.len())])
        .width() as u16;
    let width = area.width.saturating_sub(2);
    let offset = cursor.saturating_sub(width.saturating_sub(1));
    let placeholder = if app.input.is_empty() {
        "Type here, then Enter to send"
    } else {
        &app.input
    };
    f.render_widget(
        Paragraph::new(placeholder)
            .style(Style::new().fg(if app.input.is_empty() {
                ui::MUTED
            } else {
                ui::TEXT
            }))
            .scroll((0, offset))
            .block(ui::block(&title)),
        area,
    );
    if area.width > 2 && area.height > 2 {
        f.set_cursor_position((area.x + 1 + cursor.saturating_sub(offset), area.y + 1));
    }
}
