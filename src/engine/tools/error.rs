#[derive(Debug, Clone)]
pub enum DynamicToolError {
    MissingExecutable { tool: String },
    MissingPython { details: String },
    InvalidChartRequest(String),
    UnsupportedLanguage(String),
    NativeNotImplemented(String),
    ExecutionFailed { tool: String, details: String },
    TimedOut { tool: String, timeout_ms: u64 },
}

impl std::fmt::Display for DynamicToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingExecutable { tool } => write!(
                f,
                "No executable path could be resolved for tool '{}'.",
                tool
            ),
            Self::MissingPython { details } => write!(f, "Python runtime unavailable: {}", details),
            Self::InvalidChartRequest(details) => {
                write!(f, "Invalid chart request format: {}", details)
            }
            Self::UnsupportedLanguage(language) => {
                write!(f, "Unsupported tool language: {}", language)
            }
            Self::NativeNotImplemented(tool) => write!(
                f,
                "Native tool '{}' is not implemented in this build.",
                tool
            ),
            Self::ExecutionFailed { tool, details } => {
                write!(f, "Tool '{}' failed: {}", tool, details)
            }
            Self::TimedOut { tool, timeout_ms } => {
                write!(f, "Tool '{}' timed out after {} ms.", tool, timeout_ms)
            }
        }
    }
}

impl std::error::Error for DynamicToolError {}
