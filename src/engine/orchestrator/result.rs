use serde::{Deserialize, Serialize};
use crate::engine::orchestrator::formatter::format_supervisor_result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorExecutionResult {
    pub status: String,
    pub summary: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub hide_evidence: bool,
    pub error: Option<String>,
    pub raw_response: String,
}

impl SupervisorExecutionResult {
    pub fn completed(
        summary: String,
        evidence: Vec<String>,
        artifacts: Vec<String>,
        verified: bool,
        hide_evidence: bool,
        raw_response: String,
    ) -> Self {
        Self {
            status: "completed".to_string(),
            summary,
            evidence,
            artifacts,
            verified,
            hide_evidence,
            error: None,
            raw_response,
        }
    }

    pub fn failed(summary: String, error: String, raw_response: String) -> Self {
        Self {
            status: "failed".to_string(),
            summary,
            evidence: Vec::new(),
            artifacts: Vec::new(),
            verified: false,
            hide_evidence: false,
            error: Some(error),
            raw_response,
        }
    }

    pub fn blocked(
        summary: String,
        evidence: Vec<String>,
        error: String,
        raw_response: String,
    ) -> Self {
        Self {
            status: "blocked".to_string(),
            summary,
            evidence,
            artifacts: Vec::new(),
            verified: false,
            hide_evidence: false,
            error: Some(error),
            raw_response,
        }
    }

    pub fn display_text(&self) -> String {
        let structured = format_supervisor_result(self);
        serde_json::to_string(&structured).unwrap_or_else(|_| self.summary.clone())
    }
}
