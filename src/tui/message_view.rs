use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use crate::backend::Message;
use crate::engine::orchestrator::{StructuredResponse, SupervisorExecutionResult, ChartPart};

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
        for chart in res.charts {
            lines.extend(format_chart(&chart));
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

fn format_chart(c: &ChartPart) -> Vec<Line<'static>> {
    let title = c.title.as_deref().unwrap_or("Chart");
    let mut lines = vec![Line::from(Span::styled(format!("📊 [CHART: {}] ({})", title, c.chart_type), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))];
    if let (Some(lbls), Some(vals)) = (&c.labels, &c.data) {
        let is_pie = c.chart_type.to_lowercase() == "pie";
        let is_line = c.chart_type.to_lowercase() == "line";
        let sum: f64 = if is_pie { vals.iter().sum() } else { vals.iter().cloned().fold(0.0, f64::max) };
        if !is_line {
            for (lbl, &val) in lbls.iter().zip(vals.iter()) {
                let width = if sum > 0.0 { (val / sum * 20.0) as usize } else { 0 };
                let bar = "█".repeat(width);
                let val_str = if is_pie { format!(" {:.1}% ({:.1})", val / sum * 100.0, val) } else { format!(" ({:.1})", val) };
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:12}: ", lbl), Style::default().fg(Color::Yellow)),
                    Span::styled(bar, Style::default().fg(Color::Green)),
                    Span::styled(val_str, Style::default().fg(Color::White)),
                ]));
            }
        } else {
            let trend: Vec<String> = lbls.iter().zip(vals.iter()).map(|(l, v)| format!("{}({:.1})", l, v)).collect();
            lines.push(Line::from(vec![
                Span::styled("  Trend: ", Style::default().fg(Color::Yellow)),
                Span::styled(trend.join(" ➔ "), Style::default().fg(Color::White)),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines
}
