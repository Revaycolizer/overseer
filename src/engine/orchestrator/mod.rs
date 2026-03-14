mod planner;
mod dispatcher;

use rig::{
    providers::ollama,
    completion::{Message, Prompt},
    tool::Tool,
    client::CompletionClient,
};
use std::time::Duration;
use tokio::time::sleep;

use crate::engine::agents::master_planner::MasterPlanner;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

pub use planner::MacroTask;

pub async fn orchestrate_request<'a>(
    db: &crate::db::Database,
    master_planner: &'a MasterPlanner,
    all_agents: &[AgentInfo],
    query: &str, 
    _history: Vec<Message>,
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
        .replace("completed successfully", "[past success]");
        
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

        // Fetch micro-steps from domain planner if needed
        let planner_domain = format!("{}_planner", macro_task.domain);
        let micro_steps: Vec<String> = if let Some(agent_info) = all_agents.iter().find(|a| a.domain == planner_domain || a.domain == macro_task.domain && (a.name.to_lowercase().contains("planner") || a.domain == "orchestrator")) {
            println!("🧠 {} Decomposing task...", agent_info.name);
            let model_name = if agent_info.model.is_empty() { "qwen2.5:7b-instruct" } else { &agent_info.model };
            let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(model_name))
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
            
            // Deterministic shortcuts
            if step_lower.starts_with("wait") || step_lower.contains("wait for") {
                println!("⏱️ AUTO-WAIT: Pausing 2s...");
                sleep(Duration::from_secs(2)).await;
                let result = format!("Wait step completed: '{}'", step);
                plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, result));
                final_report = result;
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
                        } else {
                            None
                        };
                        let tool = crate::engine::tools::BrowserTool::new(script_path);
                        if let Ok(out) = tool.call(crate::engine::tools::BrowserArgs { action: "ping".to_string(), url: None, text: None, selector: None, submit: None, direction: None, amount: None, navigate: None, x: None, y: None }).await {
                            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(out.find('{').map(|i| &out[i..]).unwrap_or(&out)) {
                                if let Some(pid) = json_val.get("browser_pid").and_then(|v| v.as_i64()) { browser_pid = Some(pid as i32); }
                            }
                            sleep(Duration::from_secs(1)).await;
                            let result = format!("Browser ready. {}", out);
                            println!("✅ AUTO-BROWSER-OPEN: {}", out);
                            plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, result));
                            final_report = result;
                        }
                        continue;
                    } else {
                        println!("🚀 AUTO-LAUNCH: Directly launching '{}' without model...", app_name);
                        let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                        let script_path = if let Some(tinfo) = tools_list.iter().find(|t| t.name == "ui_automation") {
                            crate::engine::utils::discovery::get_tool_executable_path(tinfo)
                        } else {
                            None
                        };
                        let tool = crate::engine::tools::PyUITool::new(script_path);
                        let result_str = tool.call(crate::engine::tools::PyUIArgs { action: "launch".to_string(), app: Some(app_name.clone()), text: None, x: None, y: None, submit: None, clear: None, key: None, no_launch: None }).await.unwrap_or_else(|e| format!("Launch error: {}", e));
                        sleep(Duration::from_secs(2)).await;
                        let result = format!("Launched '{}'. Result: {}", app_name, result_str);
                        println!("✅ AUTO-LAUNCH result: {}", result_str);
                        plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, result));
                        final_report = result;
                        continue;
                    }
                }
            }

            // Route to supervisor loop methods
            match dispatcher::dispatch_domain(
                db,
                all_agents,
                &macro_task.domain,
                step,
                query,
                &memories,
                &plan_history,
                memory_context,
                &macro_task.task,
                perception_tool,
                ollama_client,
                &mut browser_pid,
            ).await {
                Ok(r) => { 
                    plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i+1, step, r)); 
                    final_report = r; 
                },
                Err(e) => { 
                    return format!("PLAN EXECUTION HALTED.\nError: {}\nLast State: {}", e, final_report); 
                }
            }
        }
    }
    
    final_report
}
