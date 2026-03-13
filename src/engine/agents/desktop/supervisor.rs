use rig::{
    agent::AgentBuilder,
    completion::{Message, Chat},
    providers::ollama,
    client::CompletionClient,
    tool::Tool,
};
use crate::engine::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::engine::agents::overseer::OverseerAgent;

#[derive(Clone)]
pub struct AppSupervisor {
    pub agent: rig::agent::Agent<ollama::CompletionModel>,
}

impl AppSupervisor {
    pub fn new(ollama_client: &ollama::Client, instructions: &str) -> Self {
        let agent = AgentBuilder::new(ollama_client.completion_model("qwen2.5:7b-instruct"))
            .preamble(instructions)
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
        db: &crate::db::Database,
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

            if let Some(tool_output) = execute_tools(db, &response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, &mut dummy_pid, &mut last_action_code, false).await {
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
