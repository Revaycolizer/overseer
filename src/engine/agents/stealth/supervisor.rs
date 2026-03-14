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

            if response.contains("done") || (!response.trim().starts_with('{') && response.len() > 20) {
                if response.contains("done") || response.trim().starts_with('{') {
                    println!("🏁 STEALTH: Requesting final summary...");
                    let final_summary = self.agent.chat("Research complete. Provide the final, verified answer to the user now in PLAIN TEXT. DO NOT output JSON. Strip away any unnecessary asterisks or markdown artifacts.", history.clone()).await
                        .unwrap_or_else(|_| "Research concluded.".to_string());
                    return Ok(final_summary.replace('*', "").trim().to_string());
                }
                return Ok(response.replace('*', "").trim().to_string());
            }

            // Redundancy Guard
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                let current_url = val.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let current_query = val.get("query").and_then(|v| v.as_str()).unwrap_or("");
                
                if (!current_url.is_empty() && (attempted_urls.contains(&current_url.to_string()) || failed_urls.contains(&current_url.to_string()))) || 
                   (!current_query.is_empty() && attempted_urls.contains(&current_query.to_string())) {
                    println!("🛡️  REDUNDANCY GUARD: Blocked repeat action for '{}'", if !current_url.is_empty() { current_url } else { current_query });
                    static_data_turns += 1;
                    
                    if static_data_turns >= 4 {
                        println!("🛑  STAGNATION LIMIT: Forcing research conclusion.");
                        let final_summary = self.agent.chat("Research stalled due to repeated actions. Provide the final, verified answer to the user now in PLAIN TEXT. DO NOT output JSON. Strip away any unnecessary asterisks or markdown artifacts.", history.clone()).await
                            .unwrap_or_else(|_| "Research concluded with partial data.".to_string());
                        return Ok(final_summary.replace('*', "").trim().to_string());
                    }

                    history.push(Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(format!(
                            "ERROR: You just tried to access a URL or query that is already in the RESEARCH STATE. DO NOT repeat actions. Try a different path or output 'done' if you have gathered enough information."
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
                    let final_summary = self.agent.chat("Research complete based on tool-indicated completion. Provide the final, verified answer to the user now in PLAIN TEXT. Strip away asterisks.", history.clone()).await
                        .unwrap_or_else(|_| "Research concluded.".to_string());
                    return Ok(final_summary.replace('*', "").trim().to_string());
                }

                // Parse for URL tracking
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response) {
                    if let Some(url) = val.get("url").and_then(|v| v.as_str()) {
                        if tool_output.contains("404") || tool_output.contains("Error") {
                            failed_urls.push(url.to_string());
                            static_data_turns += 1;
                        } else {
                            attempted_urls.push(url.to_string());
                            
                            // Check for content growth as a sign of progress
                            let stripped_output = tool_output.chars().filter(|c| c.is_alphanumeric()).collect::<String>();
                            let mut is_novel = true;
                            for past_extracted in &extracted_data {
                                if stripped_output.contains(past_extracted) || past_extracted.contains(&stripped_output) {
                                    is_novel = false;
                                    break;
                                }
                            }

                            if !stripped_output.is_empty() && is_novel {
                                // Extract a fingerprint (first 100 alphanumeric chars) to track novelty
                                let fingerprint = stripped_output.chars().take(100).collect::<String>();
                                extracted_data.push(fingerprint);
                                static_data_turns = 0;
                            } else {
                                static_data_turns += 1;
                            }
                        }
                    } else if let Some(query) = val.get("query").and_then(|v| v.as_str()) {
                        if tool_output == "[]" || tool_output.contains("no results") {
                            failed_urls.push(format!("QUERY: {}", query));
                            static_data_turns += 1;
                        } else {
                            attempted_urls.push(query.to_string());
                            static_data_turns = 0;
                        }
                    }
                }

                if static_data_turns >= 3 {
                    println!("⚠️  STAGNATION WARNING: 3 turns with no new data. Encouraging conclusion.");
                    let message = if tool_output == "[]" {
                        "ERROR: The search returned zero results. DuckDuckGo might be blocked or the query is too specific. Try broader keywords or different terms."
                    } else if tool_output.contains("404") {
                        "ERROR: That page was not found (404). Do NOT guess URLs. Use 'stealth_search' to find the current valid URL for this resource."
                    } else {
                        "NOTE: No new data points have been extracted for 3 turns. If you have enough info, output 'done'. Otherwise, try a radically different search query."
                    };
                    history.push(Message::User {
                        content: rig::OneOrMany::one(rig::message::UserContent::text(message.to_string())),
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
