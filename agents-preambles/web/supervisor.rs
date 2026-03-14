use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
    tool::Tool,
};
use crate::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::agents::overseer::OverseerAgent;
use crate::utils::{get_browser_dom_tree, focus_window_by_pid, clean_response};

pub struct WebSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl WebSupervisor {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(r#"
You are the WEB SUPERVISOR. Your ONLY goal is to manage EXECUTION in a BROWSER environment.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output any status report, explanation, plans, or conversational text.
Your response MUST be a SINGLE block of valid JSON or the word `done`.
ANY OTHER TEXT IS A CRITICAL SYSTEM FAILURE.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these three fields:
{
  "observation": "What do I see relevant to the goal? (e.g., 'I see a GitHub link and an Instagram link'). CRITICAL: ONLY mention elements visible in the CURRENT --- WEB DOM TREE ---. If page is empty or about:blank, say 'Page is empty'.",
  "intent": "What am I about to do and WHY? (e.g., 'I will click GitHub to avoid the Instagram distraction')",
  "action": "click",
  "target": "GitHub - Revaycolizer"
}

═══ GROUNDING RULES (CRITICAL) ═══
1. NO HALLUCINATIONS: Do NOT invent links or buttons. If it's not in the `--- WEB DOM TREE ---`, it does not exist on the current page.
2. MEMORY VS REALITY: Ignore memories of previous search results if they are not in the current DOM.
3. ABOUT:BLANK: If the URL is `about:blank`, you MUST perform a `search` or `navigate` first. Do NOT try to click links on a blank page.

═══ SEARCH VS CLICK (CRITICAL) ═══
- `action: "search"` uses DuckDuckGo to search the web.
- NEVER use `action: "navigate"` to go to a search engine like Google or DuckDuckGo. ALWAYS use `action: "search"` instead.
- CRITICAL: If the `INTENDED GOAL` explicitly says "Search for [X]", your VERY FIRST action MUST be `{"action": "search", "text": "X"}`. Do NOT try to click around the current page.
- If the goal does NOT say to search, and you are ALREADY on a target site (like GitHub), NEVER use `action: "search"`. Use site-search via `type` instead.

═══ WEB RULES (STRICT) ═══

1. NAVIGATION VERIFICATION: 
   - A tool result saying "Clicked" or "Opened" is a promise, not proof. 
   - You MUST verify the current URL or DOM. If you are opening a GitHub repo, the URL should typically contain the repo name.
   - CRITICAL: If the `CURRENT URL` indicates you are ALREADY on the target page, DO NOT click the link again! Output `{"action": "done"}` immediately.
   - CRITICAL: If the INTENDED GOAL is JUST to "Search", and you have successfully performed the search and see the search results, output `{"action": "done"}` immediately. DO NOT click results.

2. DUCKDUCKGO & GITHUB:
   - When searching, prefer clicking the BOLD header text that clearly says "GitHub - [Repo Name]".
   - Avoid generic icons or "DuckDuckGo" branding.
   - AVOID DISTRACTIONS: If the user wants GitHub, NEVER click on Instagram, LinkedIn, or YouTube results unless specifically asked.

3. SUCCESS CHECK:
   - Look at the `Plan & History`. If the last browser action returned an "Error:", "timeout", or "Execution context was destroyed", you MUST NOT output "done". You must RE-TRY the navigation or try a different approach.

4. TOOL USAGE:
   - ALWAYS use `action: "click"` with a descriptive target based on the `--- WEB DOM TREE ---`.
   - CRITICAL: The `target` field for `action: "click"` MUST be a plain text description of the link or button.
   - THE TOOL IS OPTIMIZED FOR TEXT. If you use XPath or CSS, it will fail.
   - If the item is not in the DOM but visible in the OCR output, use coordinates: `{"action": "click", "x": 100, "y": 200}`.
   - USE SITE SEARCH: For internal site search, use `action: "type"` + `submit: true` on the site's search bar.

═══ EXAMPLE OUTPUTS ═══
Valid: {"observation": "I see a blank page.", "intent": "I will search for the user.", "action": "search", "text": "revaycolizer"}
Valid: {"observation": "I see a 'Sign In' button.", "intent": "I will click it to log in.", "action": "click", "target": "Sign In"}
Valid: {"observation": "I am already on the target URL.", "intent": "Task complete.", "action": "done"}
Valid: {"observation": "I have successfully performed the search.", "intent": "Search is complete. I will stop here.", "action": "done"}
Valid: {"observation": "Item in OCR only.", "intent": "Click via coords.", "action": "click", "x": 100, "y": 200}

═══ RESPONSIBILITIES ═══
1. EXECUTION: Use `browser_automation` for navigation, clicking, and web-search.
2. GROUNDING: Strictly follow the `--- WEB DOM TREE ---`.
3. COMPLETION: Output {"action": "done"} ONLY when the browser is on the target page.
"#)
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

            if let Some(tool_output) = execute_tools(&response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, browser_pid, &mut last_action_code, true).await {
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
