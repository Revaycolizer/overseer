use crate::db::Database;
use super::RuntimeHealth;
use super::models::try_get_available_models;
use super::protoc::locate_protoc;

pub async fn get_runtime_health(db: &Database) -> RuntimeHealth {
    let strict_models = try_get_available_models().await.unwrap_or_default();
    let ollama_reachable = !strict_models.is_empty() || try_get_available_models().await.is_ok();
    let selected_runtime_model =
        super::super::get_setting_or(db, "default_chat_model", "qwen2.5:7b-instruct").await;
    let title_model = super::super::get_setting_or(db, "title_generator_model", "qwen2.5:7b-instruct").await;
    let python_command =
        crate::engine::utils::discovery::detect_python_command().map(|parts| parts.join(" "));
    let protoc_path = locate_protoc();
    let runtime_dir = crate::engine::utils::discovery::runtime_dir()
        .display()
        .to_string();

    let runtime_model_available = strict_models
        .iter()
        .any(|model| model == &selected_runtime_model);
    let title_model_available = strict_models.iter().any(|model| model == &title_model);

    RuntimeHealth {
        ollama_reachable,
        ollama_endpoint: "http://localhost:11434".to_string(),
        available_models: strict_models,
        selected_runtime_model,
        runtime_model_available,
        title_model,
        title_model_available,
        python_command,
        protoc_path,
        runtime_dir,
        native_runtime: true,
    }
}
