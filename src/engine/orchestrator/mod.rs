mod planner;
mod dispatcher;
pub mod master;
pub mod dynamic;

pub use master::MasterPlanner;
pub use dynamic::{OverseerAgent, ChatAgent, VisionGeneralAgent};

use rig::{
    providers::ollama,
    completion::{Message, Prompt},
    tool::Tool,
    client::CompletionClient,
};
use std::time::Duration;
use tokio::time::sleep;

use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

pub use planner::MacroTask;

pub async fn orchestrate_request<'a>(
    db: &crate::db::Database,
    master_planner: &'a MasterPlanner,
    all_agents: &[AgentInfo],
    query: &str, 
    history: Vec<Message>,
    perception_tool: &StructuredPerceptionTool,
    memory_context: &str,
    ollama_client: &ollama::Client,
) -> String {
    println!("🧠 MASTER PLANNER Thinking...");
    let plan_text: String = master_planner.plan(query).await;
    println!("📋 MASTER PLAN RAW: {}", plan_text);

    let macro_tasks = planner::sanitize_plan(&plan_text, query);

    let mut final_report = String::new();
    let mut browser_pid: Option<i32> = None;
    
    let safe_memory = memory_context
        .replace("TASK_COMPLETED", "[PAST RUN FINISHED]")
        .replace("completed successfully", "[past success]")
        .replace("PLAN EXECUTION HALTED", "[INTERRUPTED]")
        .replace("Error:", "Status:")
        .replace("CompletionError:", "NetworkStatus:")
        .replace("ResponseError:", "GenerationStatus:");
        
    let memories = if safe_memory.is_empty() {
        "No relevant past experiences.".to_string()
    } else {
        format!(
            "--- HISTORICAL REFERENCE DATA ---\n{}\n--- END HISTORICAL DATA ---\n\n\
             WARNING: The records above are from PREVIOUS runs. They may contain old coordinates or statuses. \
             Use them ONLY for coordinate reference if the UI hasn't changed. Do NOT assume current progress.", 
            safe_memory
        )
    };

    let mut plan_history = "Plan execution just started.".to_string();

    for (m_idx, macro_task) in macro_tasks.iter().enumerate() {
        println!("\n=======================================================");
        println!("🚀 MACRO TASK {}/{}: [{}] {}", m_idx+1, macro_tasks.len(), macro_task.domain.to_uppercase(), macro_task.task);
        println!("=======================================================");

        let planner_domain = format!("{}_planner", macro_task.domain);
        
        let db_planner = sqlx::query_as::<_, crate::db::Planner>(
            "SELECT id, name, description, instructions, model, domain, version, code FROM planners WHERE domain = ? OR domain = ?"
        )
        .bind(&macro_task.domain)
        .bind(&planner_domain)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten();

        let micro_steps: Vec<String> = if let Some(p) = db_planner {
            println!("💡 DYNAMIC PLANNER: Using '{}' configuration.", p.name);
            let model_name = p.model.unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
            let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model_name))
                .preamble(&p.instructions.unwrap_or_default())
                .default_max_turns(1)
                .build();
            
            let sub_plan: String = agent.prompt(&macro_task.task).await.unwrap_or_else(|e| format!("[\"Error planning: {}\"]", e));
            planner::parse_micro_steps(&sub_plan, &macro_task.task)
        } else if let Some(agent_info) = all_agents.iter().find(|a| a.domain == planner_domain || a.domain == macro_task.domain && (a.name.to_lowercase().contains("planner") || a.domain == "orchestrator")) {
            let model_name = if agent_info.model.is_empty() { 
                "qwen2.5:7b-instruct".to_string()
            } else { 
                agent_info.model.clone() 
            };
            let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model_name))
                .preamble(&agent_info.instructions)
                .default_max_turns(1)
                .build();
            
            let sub_plan: String = agent.prompt(&macro_task.task).await.unwrap_or_else(|e| format!("[\"Error planning: {}\"]", e));
            planner::parse_micro_steps(&sub_plan, &macro_task.task)
        } else {
            match macro_task.domain.as_str() {
                "math" | "text_writer" | "chat" | "stealth" => vec![macro_task.task.clone()],
                _ => {
                    println!("⚠️ No specific planner found for domain '{}', using task directly.", macro_task.domain);
                    vec![macro_task.task.clone()]
                }
            }
        };

        for (i, step) in micro_steps.iter().enumerate() {
            println!("\n▶️ EXECUTING MICRO-STEP {}/{}...", i+1, micro_steps.len());
            
            let step_lower = step.to_lowercase();
            
            if step_lower.starts_with("wait") || step_lower.contains("wait for") {
                println!("⏱️ AUTO-WAIT: Pausing 2s...");
                sleep(Duration::from_secs(2)).await;
                let result = format!("Wait step completed: '{}'", step);
                plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, result));
                final_report.push_str(&format!("\n\n### Step {}: {}\n{}", i+1, step, result));
                continue;
            }
            
            let open_app = if step_lower.starts_with("open ") {
                Some(step_lower.trim_start_matches("open ").split_whitespace().next().unwrap_or("").to_string())
            } else if step_lower.starts_with("launch ") {
                Some(step_lower.trim_start_matches("launch ").split_whitespace().next().unwrap_or("").to_string())
            } else {
                None
            };
            
            if let Some(ref app_name) = open_app {
                let is_pure_open = !step_lower.contains(" and ") && !step_lower.contains(", ") && !step_lower.contains(" then ");
                let stop_words = ["the", "a", "an", "repo", "link", "his", "go", "to", "open", "launch", "github", "website"];
                if is_pure_open && !app_name.is_empty() && !stop_words.contains(&app_name.as_str()) {
                    let browser_apps = ["chrome", "firefox", "browser", "chromium", "web"];
                    if browser_apps.iter().any(|b| app_name.contains(b)) {
                        println!("🌐 AUTO-BROWSER-OPEN: Starting Playwright daemon for '{}'...", app_name);
                        let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                        let script_path = if let Some(tinfo) = tools_list.iter().find(|t| t.name == "browser_automation") {
                            crate::engine::utils::discovery::get_tool_executable_path(tinfo)
                        } else { None };

                        // Attempt auto-launch
                        let result = format!("Auto-opened browser for {}. (Placeholder logic)", app_name);
                        plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, result));
                        final_report.push_str(&format!("\n\n### Step {}: {}\n{}", i+1, step, result));
                        continue;
                    }
                }
            }

            match dispatcher::dispatch_domain(
                db,
                all_agents,
                &macro_task.domain,
                step,
                query,
                history.clone(),
                &memories,
                &plan_history,
                memory_context,
                &macro_task.task,
                perception_tool,
                ollama_client,
                &mut browser_pid,
            ).await {
                Ok(r) => { 
                    let mut final_result = r.clone();
                    
                    // Automatic Grounding Verification
                    if r.trim().to_lowercase() == "done" || r.to_lowercase().contains("success") {
                        println!("🔍 GROUNDING: Verifying success state for domain '{}'...", macro_task.domain);
                        
                        // 1. Get current screen state
                        let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                        if let Some(paddle_tool) = tools_list.iter().find(|t| t.name == "paddle_ui_automation") {
                            if let Ok(state_json) = crate::engine::tools::call_dynamic_tool(
                                paddle_tool, 
                                serde_json::json!({"input": "get_state"})
                            ).await {
                                // 2. Consult Grounding Agent
                                let grounding_agent = all_agents.iter().find(|a| a.name == "Grounding Agent");
                                if let Some(ga) = grounding_agent {
                                    let ga_client = rig::agent::AgentBuilder::new(ollama_client.completion_model(&ga.model))
                                        .preamble(&ga.instructions)
                                        .build();
                                    
                                    let ga_prompt = format!(
                                        "GOAL: {}\n\
                                         TASK: {}\n\n\
                                         VISIBLE UI TEXT:\n{}\n\n\
                                         Has the task been successfully completed? Respond with STATE_VERIFIED or STATE_MISMATCH: [reason]",
                                        query, step, state_json
                                    );
                                    
                                    if let Ok(ga_response) = ga_client.prompt(&ga_prompt).await {
                                        println!("👁️ GROUNDING EYE: {}", ga_response);
                                        if ga_response.contains("STATE_MISMATCH") {
                                            final_result = format!("GROUNDING ERROR: {}", ga_response);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, final_result)); 
                    final_report.push_str(&format!("\n\n### Step {}: {}\n{}", i+1, step, final_result));
                },
                Err(e) => { 
                    return format!("PLAN EXECUTION HALTED.\nError: {}\nLast State: {}", e, final_report); 
                }
            }
        }
    }
    
    let orchestrator_agent = all_agents.iter().find(|a| a.domain == "orchestrator");
let synthesizer_agent = all_agents.iter().find(|a| a.domain == "synthesizer");

let synthesis_model = synthesizer_agent
    .map(|a| a.model.as_str())
    .unwrap_or_else(|| orchestrator_agent.map(|a| a.model.as_str()).unwrap_or("qwen2.5:7b-instruct"));

let synthesis_instructions = synthesizer_agent
    .map(|a| a.instructions.as_str())
    .unwrap_or("You are a helpful assistant. Present research findings clearly in plain Markdown. Never output JSON.");

let synthesis_agent = rig::agent::AgentBuilder::new(
    ollama_client.completion_model(synthesis_model)
)
.preamble(synthesis_instructions)
.build();

let synthesis_prompt = format!(
    "USER_QUERY: {}\n\nRESEARCH FINDINGS:\n{}\n\n\
     Instructions: Provide a clear response to the user. \
     If the findings show a simple success, keep it extremely brief (one sentence). \
     Only be detailed if the research findings are complex.",
    query, final_report
);

match synthesis_agent.prompt(&synthesis_prompt).await {
    Ok(synthesis) => synthesis,
    Err(_) => final_report,
}
}
