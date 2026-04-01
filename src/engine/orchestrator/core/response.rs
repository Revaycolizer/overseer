use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredResponse {
    #[serde(default)]
    pub status: bool,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub messages: Vec<MessagePart>,
    #[serde(default)]
    pub sources: Vec<SourcePart>,
    #[serde(default)]
    pub attachments: Vec<String>,
    #[serde(default)]
    pub charts: Vec<ChartPart>,
    #[serde(default)]
    pub unknown: Vec<UnknownPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessagePart {
    pub message: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourcePart {
    pub source: String,
    pub link: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChartPart {
    pub chart_type: String, // svg, html, asset, bar, line, pie
    #[serde(default)]
    pub value: Option<String>, // payload for legacy
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    #[serde(default)]
    pub data: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnknownPart {
    pub data: serde_json::Value,
    #[serde(rename = "type")]
    pub kind: String, // asset, text, image, media
}

impl StructuredResponse {
    pub fn new() -> Self {
        Self {
            status: true,
            artifacts: Vec::new(),
            messages: Vec::new(),
            sources: Vec::new(),
            attachments: Vec::new(),
            charts: Vec::new(),
            unknown: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: StructuredResponse) {
        self.status = self.status && other.status;
        
        // Add unique artifacts
        for a in other.artifacts {
            if !self.artifacts.contains(&a) { self.artifacts.push(a); }
        }

        // Add unique messages
        for m in other.messages {
            if !self.messages.iter().any(|existing| existing.message == m.message) {
                self.messages.push(m);
            }
        }

        // Add unique sources
        for s in other.sources {
            if !self.sources.iter().any(|existing| existing.description == s.description) {
                self.sources.push(s);
            }
        }

        // Add unique attachments
        for a in other.attachments {
            if !self.attachments.contains(&a) { self.attachments.push(a); }
        }

        // Add unique charts (deduplicate by title and type)
        for c in other.charts {
            if !self.charts.iter().any(|existing| existing.title == c.title && existing.chart_type == c.chart_type) {
                self.charts.push(c);
            }
        }

        for u in other.unknown {
            self.unknown.push(u);
        }
    }
}
