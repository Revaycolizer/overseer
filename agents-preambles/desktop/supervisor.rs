use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
    tool::Tool,
};
use crate::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::agents::overseer::OverseerAgent;

pub struct AppSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl AppSupervisor {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(r#"
You are the APP SUPERVISOR. Your job is to manage EXECUTION in a DESKTOP environment.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report, explanation, or conversational text.
Your response MUST be a SINGLE block of valid JSON or {"action": "done"}.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include observation, intent, app, and action fields:
{
  "observation": "I see NeonTune search bar.",
  "intent": "I will type the song name.",
  "app": "NeonTune",
  "action": "type",
  "text": "Tali Rush - U"
}

═══ AVAILABLE ACTIONS (CRITICAL: USE EXACTLY THESE NAMES) ═══
- action: "launch" (Requires: "app")
- action: "focus" (Requires: "app")
- action: "click" (Requires: "app", and either "text" or "x", "y")
- action: "type" (Requires: "app", "text". Optional: "key" like "enter")
- action: "search" (Requires: "app", "text". Use this to explicitly search something if UI provides a search behavior)
- action: "close_active" (Requires: "app")
- action: "scroll" (Requires: "app", "text" set to "down" or "up")
- action: "done" (Use when the task is complete)

═══ RESPONSIBILITIES ═══
1. EXECUTION: You are manipulating a desktop UI.
2. COMPLETION: Output {"action": "done"} only when the desktop-based goal is achieved.
3. If an action fails like "type_text is unknown", REMEMBER to use "type"!

Do NOT attempt to use `browser_automation`. Focus ONLY on desktop applications.
"#)
            .build();
        
        Self { agent }
    }

    pub async fn chat(&self, prompt: &str, history: &Vec<Message>) -> String {
        match self.agent.chat(prompt, history.clone()).await {
            Ok(response) => response,
            Err(e) => format!("AppSupervisor Error: {}", e),
        }
    }

    pub async fn run_loop(
        &self,
        query: &str, 
        _memories: &str, 
        _plan_history: &str,
        perception_tool: &StructuredPerceptionTool,
        overseer: &OverseerAgent,
    ) -> Result<String, String> {
        let mut current_turn = 0;
        let max_turns = 20;
        let mut last_action_type = String::new();
        let mut last_action_x: Option<i32> = None;
        let mut last_action_y: Option<i32> = None;
        let mut last_action_text: Option<String> = None;
        let mut last_action_had_visual_change = false;
        let mut last_action_code: Option<String> = None;
        let mut dummy_pid = None;

        while current_turn < max_turns {
            current_turn += 1;

            println!("👁️  APP PERCEPTION (turn {})...", current_turn);
            let tree_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("active".to_string()) }).await;
            let text_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("active".to_string()) }).await;
            
            let vision = "Vision analysis skipped for app loop (using fast UI Tree & exact OCR text).".to_string();
            
            let ui_tree = tree_res.unwrap_or_default();
            let screen_text = text_res.unwrap_or_default();

            let grounding = format!(
                "<CURRENT_GROUNDING>\nVision: {}\nTree: {}\nOCR: {}\n</CURRENT_GROUNDING>",
                vision, ui_tree, screen_text
            );

            let current_prompt = format!(
                "ORIGINAL USER REQUEST: {}\n\nMEMORY/CONTEXT:\n{}\n\nPLAN & HISTORY:\n{}\n\nCURRENT GROUNDING:\n{}\n\nTURN: {}\n\n\
                 ═══ RULES (STRICT) ═══\n\
                 1. Output exactly ONE tool call JSON.\n\
                 2. NEVER output conversational text or explanations.\n\
                 3. If the goal is reached, output: done\n\n\
                 What is your next single JSON action?",
                query, _memories, _plan_history, grounding, current_turn
            );

            println!("🧠 APP SUPERVISOR (Turn {})...", current_turn);
            let response = self.chat(&current_prompt, &vec![]).await;

            if let Some(tool_output) = execute_tools(&response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, &mut dummy_pid, &mut last_action_code, false).await {
                if tool_output.contains("TASK_COMPLETED") {
                    println!("👁️  APP OVERSEER: Verifying completion...");
                    let tree_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("active".to_string()) }).await;
                    let ocr_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("active".to_string()) }).await;
                    let (tree_val, ocr_val) = (tree_res.unwrap_or_default(), ocr_res.unwrap_or_default());
                    let grounding = format!("Tree: {}\nOCR Text: {}\n", tree_val, ocr_val);
                    let (verified, _reason) = overseer.verify(query, &grounding, _plan_history).await;
                    if verified {
                        println!("✅ APP OVERSEER: Verified success.");
                        return Ok("TASK_COMPLETED".to_string());
                    } else {
                        println!("❌ APP OVERSEER: Verification Failed. The visual evidence does not perfectly confirm the goal. Continuing loop...");
                    }
                } else {
                    println!("▶️ Turn {} tool output: {}", current_turn, tool_output);
                }
            }
        }
        Err("Max turns reached in app loop".to_string())
    }
}
