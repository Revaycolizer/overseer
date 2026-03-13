use rig::providers::ollama;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::tool::Tool;
use crate::engine::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::engine::agents::overseer::OverseerAgent;

#[derive(Clone)]
pub struct OfficeSupervisor {
    agent: Agent<ollama::CompletionModel>,
}

impl OfficeSupervisor {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let preamble = r#"You are the OfficeSupervisor, an elite specialized AI that creates, formats, and opens LibreOffice documents.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report, explanation, or conversational text.
Your response MUST be a SINGLE block of valid JSON or the word `done`.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these fields:
{
  "observation": "What do I see relevant to the goal? (e.g., 'I see the target URL in OCR data, I need to clone it.')",
  "intent": "What am I about to do and WHY? (e.g., 'I will clone the repo locally to prepare for PDF conversion.')",
  "action": "code_executor",
  "language": "bash",
  "command": "git clone https://github.com/Username/Repo /tmp/repo_dir"
}

CRITICAL ACTION CONSTRAINT:
- You MUST ONLY EVER use "action": "code_executor" OR "action": "done".
- DO NOT use "open_url", "navigate", or "click". You are an OFFICE agent, not a BROWSER agent.
- NEVER attempt to research or fetch data from the web. Use the files provided (e.g., /tmp/summary.txt).
- DO NOT rewrite or summarize the content again. Use `libreoffice --headless --convert-to pdf` on the target file.

═══ OFFICE RULES & MODES (STRICT) ═══

1. DATA FLOW (CRITICAL):
   - You are the FINAL step in a pipeline. Research was ALREADY done.
   - The source text should be in `/tmp/summary.txt`.
   - If `/tmp/summary.txt` is missing, check the goal for a different path.

2. BACKGROUND MODE (Conversion):
   - Use `libreoffice --headless --convert-to pdf /tmp/summary.txt --outdir /home/revaycolizer/projects/rig`.
   - SUCCESS CHECK: If the terminal says "convert ... -> ... using filter", you succeed.
   - COMPLETION: Output `{"action": "done"}` immediately after a successful conversion.

3. UI MODE (Opening):
   - Only use if the goal says "open", "view", or "show".
   - Use `nohup libreoffice [file] </dev/null >/dev/null 2>&1 &`.

CRITICAL RULES:
- ONLY output exactly ONE JSON tool call. No conversational text.
- DO NOT INVENT ACTIONS. Use only `code_executor`. "#;

        let agent = ollama_client.agent("qwen2.5:7b-instruct")
            .preamble(preamble)
            .build();
        
        Self { agent }
    }

    pub async fn step(
        &self, 
        query: &str, 
        grounding: &str, 
        turn: usize, 
        plan_history: &str,
        memory_context: &str,
    ) -> String {
        let prompt = format!(
            "ORIGINAL USER REQUEST: {}\n\nMEMORY/CONTEXT:\n{}\n\nPLAN & HISTORY:\n{}\n\nCURRENT GROUNDING:\n{}\n\nTURN: {}\n\n\
             ═══ RULES (STRICT) ═══\n\
             1. Output exactly ONE tool call JSON.\n\
             2. NEVER output conversational text or explanations.\n\
             3. If the goal is reached, output: {{\"action\": \"done\"}}\n\n\
             What is your next single JSON action?",
            query, memory_context, plan_history, grounding, turn
        );

        match self.agent.prompt(&prompt).await {
            Ok(response) => response,
            Err(e) => format!("Error: {}", e)
        }
    }

    pub async fn run_loop(
        &self,
        db: &crate::db::Database,
        overseer: &OverseerAgent,
        query: &str,
        perception_tool: &StructuredPerceptionTool,
        _ollama_client: &ollama::Client,
        plan_history: &str,
        memory_context: &str,
    ) -> Result<String, String> {
        let mut current_turn = 1;
        let max_turns = 20;
        
        let mut last_action_x: Option<i32> = None;
        let mut last_action_y: Option<i32> = None;
        let mut last_action_text: Option<String> = None;
        let mut last_action_type: String = String::new();
        let mut last_action_had_visual_change = true;
        let mut last_action_code: Option<String> = None;
        let mut dummy_pid: Option<i32> = None;

        let mut local_plan_history = plan_history.to_string();

        while current_turn <= max_turns {
            println!("\n--- OFFICE SUPERVISOR (Turn {}) ---", current_turn);

            let is_interactive = query.to_lowercase().contains("open") || query.to_lowercase().contains("view") || query.to_lowercase().contains("show");
            
            let grounding = if is_interactive || current_turn > 1 {
                println!("👁️  OFFICE PERCEPTION: Gathering Window & OCR Data...");
                let tree_res = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("active".to_string()) }).await;
                let text_res = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("active".to_string()) }).await;
                
                format!(
                    "App Hierarchy:\n{}\n\nOCR Coordinates:\n{}", 
                    tree_res.unwrap_or_else(|_| "N/A".to_string()), 
                    text_res.unwrap_or_else(|_| "N/A".to_string())
                )
            } else {
                "Background/Headless mode detected. Skipping live UI perception for initial tool call.".to_string()
            };

            let response = self.step(
                query,
                &grounding,
                current_turn,
                &local_plan_history,
                memory_context,
            ).await;

            if let Some(tool_output) = execute_tools(db, &response, query, perception_tool, &mut last_action_x, &mut last_action_y, &mut last_action_text, &mut last_action_type, &mut last_action_had_visual_change, &mut dummy_pid, &mut last_action_code, false).await {
                let is_office_success = tool_output.contains("Already succeeded") 
                    || tool_output.contains("writer_pdf_Export")
                    || (response.contains("nohup") && tool_output.contains("Code executed successfully"));
                    
                if tool_output.contains("TASK_COMPLETED") || is_office_success {
                    let query_lower = query.to_lowercase();
                    let response_lower = response.to_lowercase();
                    if query_lower.contains("background") || query_lower.contains("headless") || response_lower.contains("headless") || response_lower.contains("background") || is_office_success {
                        println!("✅ OFFICE OVERSEER: Skipped verification for background/headless/office task.");
                        return Ok("TASK_COMPLETED".to_string());
                    }
                    println!("👁️  OFFICE OVERSEER: Verifying completion...");
                    let tree_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_ui_tree".to_string(), app: Some("active".to_string()) }).await;
                    let ocr_res: Result<String, _> = perception_tool.call(PerceptionArgs { action: "get_screen_text".to_string(), app: Some("active".to_string()) }).await;
                    let grounding = format!("Tree: {}\nOCR Text: {}\n", tree_res.unwrap_or_default(), ocr_res.unwrap_or_default());
                    let (verified, _reason) = overseer.verify(query, &grounding, &local_plan_history).await;
                    if verified {
                        println!("✅ OFFICE OVERSEER: Verified success.");
                        return Ok("TASK_COMPLETED".to_string());
                    } else {
                        println!("❌ OFFICE OVERSEER: Verification Failed. Continuing loop...");
                    }
                }
                local_plan_history.push_str(&format!("\n[Turn {} action]: {}\n[Result]: {}", current_turn, response, tool_output));
            }
            current_turn += 1;
        }
        Err("Max turns reached in office loop".to_string())
    }
}
