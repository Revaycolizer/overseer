use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
    tool::Tool,
};
use crate::engine::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::engine::agents::overseer::OverseerAgent;
use crate::engine::utils::{get_browser_dom_tree, focus_window_by_pid, clean_response};

#[derive(Clone)]
pub struct WebSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl WebSupervisor {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(instructions)
            .build();
        
        Self { agent }
    }

    pub async fn chat(&self, prompt: &str, history: &Vec<Message>) -> String {
        match self.agent.chat(prompt, history.clone()).await {
            Ok(response) => response,
            Err(e) => format!("WebSupervisor Error: {}", e),
        }
    }

    pub async fn run_loop(
        &self,
        db: &crate::db::Database,
        query: &str, 
        mut history: Vec<Message>, 
        memories: &str,
        plan_history: &str,
        perception_tool: &StructuredPerceptionTool,
        _ollama_client: &ollama::Client,
        overseer: &OverseerAgent,
        browser_pid: &mut Option<i32>,
    ) -> Result<String, String> {
        let mut current_turn = 0;
        let max_turns = 10;
        let mut last_action_type = String::new();
        let mut last_action_x: Option<i32> = None;
        let mut last_action_y: Option<i32> = None;
        let mut last_action_text: Option<String> = None;
        let mut last_action_had_visual_change = false;
        let mut last_action_code: Option<String> = None;
        let mut _vision_description = "[Vision skipped for initial speed]".to_string();
        let mut static_url_count = 0;
        let mut previous_url = String::new();

        // ── INITIAL GROUNDING: Fast DOM-Only ──
        println!("👁️  WEB PERCEPTION: Fast Grounding (DOM Only)...");
        
        if let Some(pid) = *browser_pid {
            focus_window_by_pid(pid).await;
        }

        let dom_tree = get_browser_dom_tree().await.unwrap_or_else(|| "WEB DOM tree unavailable.".to_string());
        let ui_tree = format!("--- WEB DOM TREE (SEMANTIC SCENE GRAPH) ---\n{}", dom_tree);
        let screen_text = "OCR skipped for browser task.".to_string();

        println!("👁️  WEB PERCEPTION COMPLETE.");

        let mut current_prompt = format!(
            "<CURRENT_GROUNDING>\n\
             👁️  VISION ANALYSIS (General Scene): [Skipped for Turn 1]\n\n\
             🌳 WEB DOM TREE:\n{}\n\n\
             📖 RAW SCREEN TEXT: {}\n\n\
             </CURRENT_GROUNDING>\n\n\
             <PLAN_HISTORY>\n{}\n</PLAN_HISTORY>\n\n\
             <MEMORY_LEGEND>\n{}\n</MEMORY_LEGEND>\n\n\
             <PLANNING_ARTIFACT_IGNORE>\n\
             INTENDED GOAL: Step '{}'\n\
             </PLANNING_ARTIFACT_IGNORE>\n\n\
             ═══ WEB RULES ═══\n\
             1. DOM GROUNDING: Use the WEB DOM TREE to find links and buttons.\n\
             2. NAVIGATION: If the goal is navigation, use `action: \"navigate\"` or search.\n\
             3. ONE ACTION: Output exactly ONE tool call JSON.\n\
             4. VISUAL CHANGE: If `visual_change: true` was reported, assume success.", 
            ui_tree, screen_text, plan_history, memories, query
        );

        loop {
            if current_turn >= max_turns { return Err("Max turns reached without completion.".to_string()); }
            current_turn += 1;

            if current_turn > 1 {
                if let Some(pid) = *browser_pid { focus_window_by_pid(pid).await; }
                println!("👁️  WEB PERCEPTION: Refreshing grounding (turn {})...", current_turn);
                
                let tree_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("active".to_string()) }).await;
                let text_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("active".to_string()) }).await;
                
                let fresh_vision = "Vision analysis skipped for web loop (using exact DOM).".to_string();
                _vision_description = fresh_vision.clone();

                let mut fresh_tree = tree_res.unwrap_or_default();
                let fresh_text = text_res.unwrap_or_default();

                if let Some(dom) = get_browser_dom_tree().await {
                    fresh_tree = format!("--- WEB DOM TREE ---\n{}", dom);
                }

                current_prompt = format!(
                    "<CURRENT_GROUNDING>\n\
                     👁️  VISION ANALYSIS:\n{}\n\n\
                     CURRENT URL: {}\n\n\
                     🌳 WEB DOM TREE:\n{}\n\n\
                     📖 RAW SCREEN TEXT:\n{}\n\n\
                     </CURRENT_GROUNDING>\n\n\
                     ═══ WEB RULES (STRICT) ═══\n\
                     1. Output exactly ONE tool call JSON (e.g. {{ \"action\": \"click\", \"target\": \"...\" }}).\n\
                     2. NEVER output conversational text, explanations, or Markdown guides.\n\
                     3. If the goal is reached, output: done\n\n\
                     [ROUTING HINT: BROWSER]\n\
                     INTENDED GOAL: Step '{}' (Turn {})\n", 
                    fresh_vision, previous_url, fresh_tree, fresh_text, query, current_turn
                );
            }

            let response = self.chat(&current_prompt, &history).await;
            let response = clean_response(response);
            println!("--- WEB SUPERVISOR (Turn {}) ---\n{}\n-------------------", current_turn, response);

            history.push(Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(rig::message::AssistantContent::text(response.clone())),
            });

            if let Some(tool_output) = execute_tools(db, &response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, browser_pid, &mut last_action_code, true).await {
                println!("--- TOOL OUTPUT ---\n{}\n-------------------", tool_output);
                
                // Track static URL
                let mut current_url = previous_url.clone();
                for line in tool_output.lines() {
                    if line.trim().starts_with('{') {
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                            if let Some(url) = json_val.get("url").and_then(|v| v.as_str()) {
                                current_url = url.to_string();
                            }
                        }
                    }
                }
                // Ignore hash changes for static url count
                let base_current = current_url.split('#').next().unwrap_or(&current_url);
                let base_prev = previous_url.split('#').next().unwrap_or(&previous_url);
                
                if base_current == base_prev && !tool_output.contains("TASK_COMPLETED") {
                    static_url_count += 1;
                } else {
                    static_url_count = 0;
                    previous_url = current_url.clone();
                }
                
                let mut final_output = tool_output.clone();
                
                // Auto-resolve search steps
                let is_search_goal = query.to_lowercase().starts_with("search ") || query.to_lowercase().contains("search for");
                if is_search_goal && last_action_type == "search" && (final_output.contains("search_submitted") || final_output.contains("Search complete")) {
                    println!("✅ WEB SUPERVISOR AUTO-COMPLETE: Search action finished for a search goal.");
                    return Ok("TASK_COMPLETED".to_string());
                }

                if static_url_count >= 3 {
                    println!("🚫 STATIC URL WARNING: URL has not changed for {} turns. Force-completing step!", static_url_count);
                    final_output = "TASK_COMPLETED".to_string();
                } else if static_url_count == 2 {
                    println!("🚫 STATIC URL WARNING: URL has not changed for 2 turns.");
                    final_output = format!("{}\n\nWARNING: The browser URL '{}' has not changed for 2 turns. You are likely on the correct page. If the intended goal of this step is already achieved (e.g. you navigated successfully or you are already on the target page), you MUST output {{ \"action\": \"done\" }} immediately. Do not keep repeatedly clicking on the same page.", final_output, current_url);
                }

                if final_output.contains("AUTO_COMPLETED") { return Ok("TASK_COMPLETED".to_string()); }
                if final_output.contains("TASK_COMPLETED") {
                    println!("👁️  WEB OVERSEER: Verifying completion...");
                    let dom_tree = get_browser_dom_tree().await.unwrap_or_else(|| "WEB DOM tree unavailable.".to_string());
                    let grounding = format!("🌳 WEB DOM TREE:\n{}\n", dom_tree);
                    let mut history_str = String::new();
                    for msg in &history {
                        history_str.push_str(&format!("{:?}\n", msg));
                    }
                    let (verified, reason) = overseer.verify(query, &grounding, &history_str).await;
                    if verified {
                        println!("✅ WEB OVERSEER: Verified success.");
                        return Ok(final_output);
                    } else if static_url_count >= 3 {
                        println!("⚠️ WEB OVERSEER: Verification rejected, but URL is static for 3 turns. Force-exiting loop to prevent infinite loop.");
                        return Ok(final_output);
                    } else {
                        println!("❌ WEB OVERSEER: Completion rejected: {}", reason);
                        history.push(Message::User {
                            content: rig::OneOrMany::one(rig::message::UserContent::text(format!(
                                "OVERSEER REJECTION: You claimed this step is done, but verification failed. Reason: {}\nDo NOT output 'done' again until the goal is reached.", reason
                            ))),
                        });
                        continue;
                    }
                }
                
                history.push(Message::User {
                    content: rig::OneOrMany::one(rig::message::UserContent::text(format!("Tool Results:\n{}", final_output))),
                });
            } else {
                if response.contains("done") || response.contains("TASK_COMPLETED") { return Ok(response); }
                history.push(Message::User {
                    content: rig::OneOrMany::one(rig::message::UserContent::text("CRITICAL: You MUST use a JSON tool call.".to_string())),
                });
            }
        }
    }
}
