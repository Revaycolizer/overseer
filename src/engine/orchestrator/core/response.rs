use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructuredResponse {
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
            artifacts: Vec::new(),
            messages: Vec::new(),
            sources: Vec::new(),
            attachments: Vec::new(),
            charts: Vec::new(),
            unknown: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: StructuredResponse) {
        self.artifacts.extend(other.artifacts);
        self.messages.extend(other.messages);
        self.sources.extend(other.sources);
        self.attachments.extend(other.attachments);
        self.charts.extend(other.charts);
        self.unknown.extend(other.unknown);
        
        // Remove duplicates where sensible
        self.attachments.dedup();
        self.artifacts.dedup();
    }
}
