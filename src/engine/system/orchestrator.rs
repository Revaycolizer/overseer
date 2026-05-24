// src/engine/system/orchestrator.rs
use crate::db::Database;
use crate::engine::system::AgentSystem;
use crate::engine::orchestrator::planning::master::MasterPlanner;
use crate::engine::tools::StructuredPerceptionTool;

pub async fn orchestrate_request(
    system: &AgentSystem,
    db: &Database,
    conversation_id: i64,
    user_query: &str,
) -> String {
    let messages = crate::backend::get_messages(db, conversation_id)
        .await
        .unwrap_or_default();

    let history = super::history_builder::build_history(
        db, &system.ollama_client, messages, user_query
    ).await;

    let all_agents = crate::backend::get_agents(db).await.unwrap_or_default();
    let planner_model = crate::backend::resolve_model_name(
        db, None, "planner_model", "qwen3:latest"
    ).await;
    let instructions = crate::backend::get_setting(db, "master_planner_instructions").await;
    
    let master_planner = MasterPlanner::new(
        &system.ollama_client,
        &planner_model,
        &instructions,
        &all_agents,
    );

    let perception_tool = StructuredPerceptionTool::new(db.clone());
    let memory_context = crate::backend::get_setting(db, "system_memory_context").await;

    crate::engine::orchestrator::core::main_loop::orchestrate_request(
        db,
        conversation_id,
        &master_planner,
        &all_agents,
        user_query,
        history,
        &perception_tool,
        &memory_context,
        &system.ollama_client,
    ).await
}
