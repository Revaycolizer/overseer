use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
};
use regex;
use crate::engine::utils::clean_response;

#[derive(Clone)]
pub struct StealthSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl StealthSupervisor {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(instructions)
            .build();
        
        Self { agent }
    }

    pub async fn run_loop(
        &self,
        db: &crate::db::Database,
        goal: &str,
        mut history: Vec<Message>,
        memory_context: &str,
        perception_tool: &crate::engine::tools::StructuredPerceptionTool,
    ) -> Result<String, String> {
        let mut current_turn = 0;
        let max_turns = 15;
        let mut last_action_type = String::new();
        let mut last_action_x: Option<i32> = None;
        let mut last_action_y: Option<i32> = None;
        let mut last_action_text: Option<String> = None;
        let mut last_action_had_visual_change = false;
        let mut last_action_code: Option<String> = None;
        let mut browser_pid: Option<i32> = None;
        
        let mut attempted_urls: Vec<String> = Vec::new();
        let mut failed_urls: Vec<String> = Vec::new();
        let mut extracted_data: Vec<String> = Vec::new();
        let mut static_data_turns = 0;
        
        println!("👤 STEALTH RESEARCH START: {}", goal);

        loop {
            if current_turn >= max_turns {
                return Err("Max research turns reached.".to_string());
            }
            current_turn += 1;

            let attempt_str = if attempted_urls.is_empty() { "None".to_string() } else { attempted_urls.join(", ") };
            let failed_str = if failed_urls.is_empty() { "None".to_string() } else { failed_urls.join(", ") };
            let data_str = if extracted_data.is_empty() { "No hard data points found yet.".to_string() } else { extracted_data.join(" | ") };

            let prompt = format!(
                "Goal: {}\n\n\
                 [TURN {}/{}]\n\
                 ═══ RESEARCH STATE ═══\n\
                 SUCCESSFUL URLS: {}\n\
                 FAILED URLS (DO NOT RETRY): {}\n\
                 DATA EXTRACTED: {}\n\n\
                 ═══ RULES ═══\n\
                 1. Output ONE JSON tool call.\n\
                 2. DO NOT REUSE any URL or QUERY from the RESEARCH STATE.\n\
                 3. If the data needed is already EXTRACTED and verified, output 'done'.\n\
                 4. User memory: {}\n",
                goal, current_turn, max_turns, attempt_str, failed_str, data_str, memory_context
            );

            let response = match self.agent.chat(&prompt, history.clone()).await {
                Ok(r) => clean_response(r),
                Err(e) => return Err(format!("Stealth Model Error: {}", e)),
            };

            println!("--- Turn {} ---\n{}\n--------------", current_turn, response);

            if response.contains("done") || (!response.trim().starts_with('{') && response.len() > 30) {
                if response.contains("done") {
                    let final_summary = self.agent.chat("Research complete. Provide the final, verified answer to the user now in plain text based on all research turns.", history.clone()).await
                        .unwrap_or_else(|_| "Research concluded.".to_string());
                    return Ok(final_summary);
                }
                return Ok(response);
            }

            // Redundancy Guard
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                let current_url = val.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let current_query = val.get("query").and_then(|v| v.as_str()).unwrap_or("");
                
                if (!current_url.is_empty() && (attempted_urls.contains(&current_url.to_string()) || failed_urls.contains(&current_url.to_string()))) || 
                   (!current_query.is_empty() && attempted_urls.contains(&current_query.to_string())) {
                    println!("🛡️  REDUNDANCY GUARD: Blocked repeat action for '{}'", if !current_url.is_empty() { current_url } else { current_query });
                    static_data_turns += 1;
                    
                    if static_data_turns >= 5 {
                        println!("🛑  STAGNATION LIMIT: Forcing research conclusion.");
                        let final_summary = self.agent.chat("Research stalled due to repeated actions. Provide the final, verified answer to the user now in plain text based on all research turns.", history.clone()).await
                            .unwrap_or_else(|_| "Research concluded with partial data.".to_string());
                        return Ok(final_summary);
                    }

                    history.push(Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(format!(
                            "ERROR: You just tried to access a URL or query that is already in the RESEARCH STATE. DO NOT repeat actions. Try a different path or output 'done' if you are stuck."
                        ))),
                    });
                    continue;
                }
            }

            history.push(Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(rig::message::AssistantContent::text(response.clone())),
            });

            // Execute tool
            if let Some(tool_output) = crate::engine::tools::execute_tools(
                db,
                &response, goal, perception_tool, 
                &mut last_action_x, &mut last_action_y, &mut last_action_text, 
                &mut last_action_type, &mut last_action_had_visual_change, &mut browser_pid, &mut last_action_code, false
            ).await {
                println!("--- TOOL OUTPUT ---\n{}\n-------------------", tool_output);

                if tool_output.contains("TASK_COMPLETED") {
                    let final_summary = self.agent.chat("Research complete. Final summary please.", history.clone()).await
                        .unwrap_or_else(|_| "Research concluded.".to_string());
                    return Ok(final_summary);
                }

                // Parse for URL tracking
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                    if let Some(url) = val.get("url").and_then(|v| v.as_str()) {
                        if tool_output.contains("404") || tool_output.contains("Error") {
                            failed_urls.push(url.to_string());
                        } else {
                            attempted_urls.push(url.to_string());
                            
                            // Improved heuristic extraction for commit counts
                            let re = regex::Regex::new(r"(?i)(\d+)\s+Commits?").unwrap();
                            let mut found_new_data = false;
                            for cap in re.captures_iter(&tool_output) {
                                let count = cap.get(1).map_or("", |m| m.as_str());
                                let repo = url.split('/').last().unwrap_or("unknown");
                                let data_point = format!("{}: {} commits", repo, count);
                                if !extracted_data.contains(&data_point) {
                                    extracted_data.push(data_point);
                                    found_new_data = true;
                                }
                            }

                            if !found_new_data {
                                static_data_turns += 1;
                            } else {
                                static_data_turns = 0;
                            }
                        }
                    } else if let Some(query) = val.get("query").and_then(|v| v.as_str()) {
                        attempted_urls.push(query.to_string());
                    }
                }

                if static_data_turns >= 3 {
                    println!("⚠️  STAGNATION WARNING: 3 turns with no new data. Encouraging conclusion.");
                    history.push(Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text("NOTE: No new data points have been extracted for 3 turns. If you have enough information to answer the goal, output 'done'. If not, try a radically different methodology.")),
                    });
                } else {
                    history.push(Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(format!("Tool Results:\n{}", tool_output))),
                    });
                }
            } else {
                history.push(Message::User {
                    content: rig::OneOrMany::one(rig::message::UserContent::text("CRITICAL: You MUST use a JSON tool call or provide the final answer.".to_string())),
                });
            }
        }
    }
}
