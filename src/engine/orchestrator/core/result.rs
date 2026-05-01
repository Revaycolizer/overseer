use serde::{Deserialize, Serialize};
use crate::engine::orchestrator::utils::formatter::format_supervisor_result;
use super::response::MessagePart;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorExecutionResult {
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub messages: Vec<MessagePart>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub charts: Vec<super::response::ChartPart>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub hide_evidence: bool,
    #[serde(default)]
    pub report_style: Option<String>,
    pub error: Option<String>,
    pub raw_response: String,
    #[serde(flatten)]
    pub findings: std::collections::HashMap<String, serde_json::Value>,
}

impl SupervisorExecutionResult {
    pub fn new(
        status: String,
        summary: String,
        messages: Vec<MessagePart>,
        evidence: Vec<String>,
        artifacts: Vec<String>,
        charts: Vec<super::response::ChartPart>,
        verified: bool,
        hide_evidence: bool,
        report_style: Option<String>,
        raw_response: String,
        findings: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            status,
            summary,
            messages,
            evidence,
            artifacts,
            charts,
            verified,
            hide_evidence,
            report_style,
            error: None,
            raw_response,
            findings,
        }
    }

    pub fn completed(
        summary: String,
        messages: Vec<MessagePart>,
        evidence: Vec<String>,
        artifacts: Vec<String>,
        charts: Vec<super::response::ChartPart>,
        verified: bool,
        hide_evidence: bool,
        report_style: Option<String>,
        raw_response: String,
        findings: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        Self::new("completed".to_string(), summary, messages, evidence, artifacts, charts, verified, hide_evidence, report_style, raw_response, findings)
    }

    pub fn failed(
        summary: String,
        error: String,
        charts: Vec<super::response::ChartPart>,
        raw_response: String,
        report_style: Option<String>,
    ) -> Self {
        Self {
            status: "failed".to_string(),
            summary,
            messages: Vec::new(),
            evidence: Vec::new(),
            artifacts: Vec::new(),
            charts,
            verified: false,
            hide_evidence: false,
            report_style,
            error: Some(error),
            raw_response,
            findings: std::collections::HashMap::new(),
        }
    }

    pub fn blocked(
        summary: String,
        evidence: Vec<String>,
        error: String,
        charts: Vec<super::response::ChartPart>,
        raw_response: String,
        report_style: Option<String>,
    ) -> Self {
        Self {
            status: "blocked".to_string(),
            summary,
            messages: Vec::new(),
            evidence,
            artifacts: Vec::new(),
            charts,
            verified: false,
            hide_evidence: false,
            report_style,
            error: Some(error),
            raw_response,
            findings: std::collections::HashMap::new(),
        }
    }

    pub fn display_text(&self) -> String {
        let structured = format_supervisor_result(self);
        serde_json::to_string(&structured).unwrap_or_else(|_| self.summary.clone())
    }
}
