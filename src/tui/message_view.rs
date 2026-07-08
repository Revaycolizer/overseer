use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use crate::backend::Message;
use crate::engine::orchestrator::{StructuredResponse, SupervisorExecutionResult};

pub fn format_message(m: &Message) -> Vec<Line<'static>> {
    let role_style = if m.role == "user" {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
    };

    if m.role == "user" {
        return vec![Line::from(vec![
            Span::styled("[USER] ", role_style),
            Span::styled(m.content.clone(), Style::default().fg(Color::White)),
        ])];
    }

    let mut structured: Option<StructuredResponse> = serde_json::from_str(&m.content).ok();

    if structured.is_none() {
        if let Ok(res) = serde_json::from_str::<SupervisorExecutionResult>(&m.content) {
            structured = Some(crate::engine::orchestrator::utils::formatter::format_supervisor_result(&res));
        }
    }

    if let Some(res) = structured {
        let mut lines = Vec::new();
        for sub_msg in res.messages {
            let sub_role_style = if sub_msg.role == "user" {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("[{}] ", sub_msg.role.to_uppercase()), sub_role_style),
                Span::styled(sub_msg.message, Style::default().fg(Color::White)),
            ]));
        }
        if !res.artifacts.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("[ARTIFACTS] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(res.artifacts.join(", "), Style::default().fg(Color::Cyan)),
            ]));
        }
        lines
    } else {
        vec![Line::from(vec![
            Span::styled(format!("[{}] ", m.role.to_uppercase()), role_style),
            Span::styled(m.content.clone(), Style::default().fg(Color::White)),
        ])]
    }
}
