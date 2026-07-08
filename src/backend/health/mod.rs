pub mod models;
pub mod protoc;
pub mod runtime;

pub use models::*;
pub use protoc::*;
pub use runtime::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RuntimeHealth {
    pub ollama_reachable: bool,
    pub ollama_endpoint: String,
    pub available_models: Vec<String>,
    pub selected_runtime_model: String,
    pub runtime_model_available: bool,
    pub title_model: String,
    pub title_model_available: bool,
    pub python_command: Option<String>,
    pub protoc_path: Option<String>,
    pub runtime_dir: String,
    pub native_runtime: bool,
}
