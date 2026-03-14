use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
    tool::Tool,
};
use crate::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::agents::overseer::OverseerAgent;
use regex as re;

pub struct NeontuneSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl NeontuneSupervisor {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(r#"
You are the NEONTUNE SUPERVISOR. Your ONLY goal is to manage EXECUTION specifically for the NeonTune music player desktop application.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output any status report, explanation, plans, or conversational text.
Your response MUST be a SINGLE block of valid JSON or the word `done`.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these fields:
{
  "observation": "What do I see relevant to the goal? (e.g., 'I see a search field and a list of songs'). CRITICAL: ONLY mention elements visible in the CURRENT --- NEONTUNE UI TREE --- or OCR.",
  "intent": "What am I about to do and WHY? (e.g., 'I will click the first result to play the song')",
  "action": "click",
  "app": "NeonTune",
  "text": "Tali Rush - U"
}

═══ GROUNDING RULES (CRITICAL) ═══
1. NO HALLUCINATIONS: Do NOT invent buttons. If it's not in the UI Tree or OCR, it doesn't exist.
2. RECOVERY: If a previous action failed (e.g., "Text not found"), YOU MUST acknowledge this in your `observation` and try a different approach (e.g., search again or use different keywords).
3. SEARCH VS CLICK: 
   - ALWAYS use `action: "search"` for finding songs. It is a hybrid tool that finds and focuses the search bar.
   - After searching, if you see the result in the list, use `action: "click"` on the song name.

═══ NEONTUNE UI KNOWLEDGE ═══
- The window title is usually "tao window".
- Search field text: "Search music."
- "Playing Now" section indicates successful playback.
- If you see a "Pause" button, music is already playing.

═══ AVAILABLE ACTIONS ═══
- `action: "search"`: (Hybrid) Finds search bar, types, and presses enter. Requires: "app", "text" (query).
- `action: "click"`: Clicks on text. Requires: "app", "text".
- `action: "key"`: Presses a key. Requires: "app", "text" (e.g., "enter", "space").
- `action: "done"`: Use when the song is confirmed to be playing.

═══ VERBATIM RULE ═══
Use EXACT song titles provided in the goal. Do NOT truncate.
"#)
            .build();
        
        Self { agent }
    }

    pub async fn chat(&self, prompt: &str, history: &Vec<Message>) -> String {
        match self.agent.chat(prompt, history.clone()).await {
            Ok(response) => response,
            Err(e) => format!("NeontuneSupervisor Error: {}", e),
        }
    }

    pub async fn run_loop(
        &self,
        overseer: &OverseerAgent,
        query: &str,
        perception_tool: &StructuredPerceptionTool,
        plan_history: &str,
        macro_goal: &str,
        micro_step: &str,
    ) -> Result<String, String> {
        let mut current_turn = 1;
        let max_turns = 15;
        
        let mut last_action_x: Option<i32> = None;
        let mut last_action_y: Option<i32> = None;
        let mut last_action_text: Option<String> = None;
        let mut last_action_type: String = String::new();
        let mut last_action_had_visual_change = true;
        let mut last_action_code: Option<String> = None;
        let mut dummy_pid: Option<i32> = None;

        let mut local_plan_history = plan_history.to_string();

        while current_turn <= max_turns {
            println!("\n🎵 NEONTUNE SUPERVISOR (Turn {}) ---", current_turn);

            // --- OPTIMIZED PERCEPTION (OCR FIRST) ---
            println!("👁️  NEONTUNE PERCEPTION: Gathering OCR Data...");
            let text_res = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("neontune".to_string()) }).await.map_err(|e| e.to_string());
            
            // Extract plain text from JSON result
            let mut fast_path_response = None;
            let ocr_text = text_res.as_ref().ok().cloned().unwrap_or_default();
            let filtered_ocr = if ocr_text.trim().starts_with('[') {
                // It's JSON! Extract "text" fields.
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&ocr_text) {
                    if let Some(arr) = json.as_array() {
                        arr.iter()
                           .filter_map(|e| e.get("text").and_then(|t| t.as_str()))
                           .collect::<Vec<_>>()
                           .join("\n")
                    } else {
                        ocr_text.clone()
                    }
                } else {
                    ocr_text.clone()
                }
            } else {
                let filtered: Vec<_> = ocr_text.lines()
                    .filter(|line: &&str| !line.trim().starts_with('[') && !line.contains("ppocr"))
                    .collect();
                filtered.join("\n")
            };
            let ocr_lower = filtered_ocr.to_lowercase();
            // Clean OCR for more robust placeholder matching
            let ocr_clean = ocr_lower.replace(".", "").replace(",", "").replace(":", "").replace(";", "").replace("!", "").replace("?", "");

            // 1. Heuristic Extraction of Song Target
            let mut song_target = {
                let re = re::Regex::new(r"(?i)(?:play|search for)\s+(.*?)(?:\s+in\s+neontune|\s+in\s+|$|\.)").unwrap();
                
                // Extract segments from both query and macro_goal
                let mut query_quotes = Vec::new();
                for (i, part) in query.split('\'').enumerate() {
                    if i % 2 == 1 {
                        let s = part.trim().to_string();
                        if !s.is_empty() && !query_quotes.iter().any(|q: &String| q.to_lowercase() == s.to_lowercase()) {
                            query_quotes.push(s);
                        }
                    }
                }

                let mut macro_quotes = Vec::new();
                for (i, part) in macro_goal.split('\'').enumerate() {
                    if i % 2 == 1 {
                        let s = part.trim().to_string();
                        if !s.is_empty() && !macro_quotes.iter().any(|q: &String| q.to_lowercase() == s.to_lowercase()) {
                            macro_quotes.push(s);
                        }
                    }
                }

                // Choice: Use the most descriptive source
                let joined_macro = macro_quotes.join(" - ");
                let joined_query = query_quotes.join(" - ");

                if joined_macro.len() > joined_query.len() && !joined_macro.is_empty() {
                    joined_macro
                } else if !joined_query.is_empty() {
                    joined_query
                } else if let Some(caps) = re.captures(query) {
                    caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
                } else if let Some(caps) = re.captures(&format!("{} {}", macro_goal, micro_step)) {
                    caps.get(1).map(|m| m.as_str().to_string()).unwrap_or_default()
                } else {
                    query.to_string()
                }
            };

            // Deep cleanup of known noise - PRESERVE formatting like hyphens
            song_target = song_target
                .replace("Open Neontune,", "")
                .replace("Open Neontune", "")
                .replace("in Neontune", "")
                .replace("in neontune", "")
                .replace("Search for ", "")
                .replace("search for ", "")
                .replace("the song ", "")
                .replace(" song ", " ")
                .replace("'", "")
                .replace("\"", "")
                .replace(",", "")
                .trim().to_string();
                
            if !song_target.is_empty() {
                let song_lower = song_target.to_lowercase();
                
                // Check for playback indicators
                let is_playing = ocr_lower.contains("pause") 
                                 || ocr_lower.contains("playing now") 
                                 || ocr_lower.contains("now playing") 
                                 || ocr_lower.contains("playback") 
                                 || (ocr_lower.contains(" 0:") && !ocr_lower.contains(" 0:00"));

                if is_playing {
                    println!("🚀 FAST-PATH: Playback detected for '{}'. Finishing.", song_target);
                    fast_path_response = Some("done".to_string());
                } else if (ocr_clean.contains("search music") || ocr_clean.contains("search..") || ocr_clean.contains("search")) && current_turn < 3 {
                    let already_searched = local_plan_history.contains("\"action\": \"search\"") && local_plan_history.contains(&song_target);
                    if already_searched {
                        println!("🚀 FAST-PATH: Already searched for '{}'. Waiting for results...", song_target);
                    } else {
                        println!("🚀 FAST-PATH: Search bar detected. Searching for '{}'...", song_target);
                        fast_path_response = Some(format!(r#"{{"observation": "Search bar is available.", "intent": "Quick-search the song", "action": "search", "app": "neontune", "text": "{}"}}"#, song_target));
                    }
                } else if ocr_lower.contains(&song_lower) || ocr_clean.contains(&song_lower.replace(" ", "")) {
                    // Heuristic: avoid matching the song name inside the placeholder if it's very short
                    let is_placeholder_match = song_lower.len() < 4 && ocr_clean.contains("search music") && !ocr_clean.replace("search music", "").contains(&song_lower);
                    
                    if !is_placeholder_match {
                        let already_clicked = local_plan_history.contains("\"action\": \"click\"") && local_plan_history.contains(&song_target);
                        
                        if already_clicked {
                            println!("🚀 FAST-PATH: Song '{}' already clicked. Finalizing.", song_target);
                            fast_path_response = Some("done".to_string());
                        } else {
                            println!("🚀 FAST-PATH: Song match '{}' found. Clicking.", song_target);
                            fast_path_response = Some(format!(r#"{{"observation": "I see {} in results.", "intent": "Click result to play", "action": "click", "app": "neontune", "text": "{}"}}"#, song_target, song_target));
                        }
                    }
                }
            }

            let response: String = if let Some(fp) = fast_path_response {
                fp
            } else {
                println!("👁️  NEONTUNE PERCEPTION: Fast-path failed. Gathering UI Tree...");
                let tree_res = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("neontune".to_string()) }).await.map_err(|e| e.to_string());
                
                let grounding = format!(
                    "<OVERALL_GOAL>: {}\n<CURRENT_STEP>: {}\n<CURRENT_GROUNDING>:\nTree: {}\nOCR: {}", 
                    macro_goal, micro_step, tree_res.unwrap_or_default(), ocr_text
                );
                self.chat(&grounding, &Vec::new()).await
            };

            if response.contains("\"action\": \"done\"") || response.trim() == "done" {
                println!("👁️  NEONTUNE OVERSEER: Verifying completion...");
                let tree_res = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("neontune".to_string()) }).await.map_err(|e| e.to_string());
                let ocr_res = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("neontune".to_string()) }).await.map_err(|e| e.to_string());
                let (tree_val, ocr_val) = (tree_res.unwrap_or_default(), ocr_res.unwrap_or_default());
                let grounding = format!("Tree: {}\nOCR Text: {}\n", tree_val, ocr_val);
                let (verified, _reason) = overseer.verify(macro_goal, &grounding, &local_plan_history).await;
                if verified {
                    println!("✅ NEONTUNE OVERSEER: Verified success.");
                    return Ok("TASK_COMPLETED".to_string());
                } else {
                    println!("❌ NEONTUNE OVERSEER: Verification Failed. Continuing loop...");
                }
            }

            if let Some(tool_output) = execute_tools(&response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, &mut dummy_pid, &mut last_action_code, false).await {
                local_plan_history.push_str(&format!("\n[Turn {} action]: {}\n[Result]: {}", current_turn, response, tool_output));
            }

            current_turn += 1;
        }
        Err("Max turns reached in Neontune loop".to_string())
    }
}
