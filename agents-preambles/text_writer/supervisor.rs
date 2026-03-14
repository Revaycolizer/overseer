use rig::providers::ollama;
use rig::agent::Agent;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::tool::Tool;
use crate::tools::{StructuredPerceptionTool, PerceptionArgs, execute_tools};
use crate::agents::overseer::OverseerAgent;
use crate::utils::get_browser_dom_tree;

pub struct TextWriterSupervisor {
    agent: Agent<ollama::CompletionModel>,
}

impl TextWriterSupervisor {
    pub fn new(ollama_client: &ollama::Client) -> Self {
        let text_writer_prompt = r#"You are the TextWriterSupervisor, an elite specialized AI that reads screen content and writes text files.

═══ RULE 0: NO CONVERSATION (CRITICAL) ═══
NEVER output a status report or explanation.
Your response MUST be a SINGLE block of valid JSON or the word `done`.

═══ JSON STRUCTURE (STRICT) ═══
Every tool call MUST include these three fields:
{
  "observation": "What text do I see in CURRENT GROUNDING relevant to the goal?",
  "intent": "What am I about to write to the file?",
  "action": "code_executor",
  "language": "bash",
  "command": "cat << 'EOF' > /tmp/summary.txt\nThis is the summary text.\nEOF"
}

═══ TEXT WRITER RULES (STRICT) ═══
1. READ: Look at the `CURRENT GROUNDING` to find the information requested by your goal. CRITICAL: For GitHub repositories, prioritize reading the README or file list. Ignore \"No description, website, or topics\" sidebar notes if real content exists.
2. SYNTHESIZE: Write a comprehensive, helpful summary or response based ONLY on the grounding data. Extract the core purpose and functionality.
3. WRITE: Use `code_executor` and specify `"language": "bash"` to write your summary to the requested file using a bash heredoc (`cat << 'EOF' > /path/to/file ...`).
4. COMPLETION: If the result of the `code_executor` shows SUCCESS (e.g., no errors), you MUST output `{"action": "done"}` on the next turn.

CRITICAL:
- ONLY output exactly ONE JSON tool call. No conversational text."#;

        let agent = ollama_client.agent("qwen2.5:7b-instruct")
            .preamble(text_writer_prompt)
            .build();
        
        Self { agent }
    }

    pub async fn step(
        &self, 
        query: &str, 
        grounding: &str, 
        turn: usize, 
        plan_history: &str
    ) -> Result<String, String> {
        let user_prompt = format!(
            "--- TURN {} ---\nINTENDED GOAL: {}\n\nCURRENT GROUNDING (OCR/DOM data):\n{}\n\nPAST PLAN HISTORY/CONTEXT:\n{}\n\nWhat is your next JSON tool call?",
            turn, query, grounding, plan_history
        );

        match self.agent.prompt(&user_prompt).await {
            Ok(response) => Ok(response.trim().to_string()),
            Err(e) => Err(format!("Ollama error: {}", e))
        }
    }

    pub async fn run_loop(
        &self,
        _overseer: &OverseerAgent,
        goal: &str,
        perception_tool: &StructuredPerceptionTool,
        _ollama_client: &ollama::Client,
        plan_history: &str,
        _memory_context: &str,
    ) -> Result<String, String> {
        let mut turns = 0;
        let max_turns = 3;
        
        while turns < max_turns {
            turns += 1;
            println!("\n--- TEXT WRITER SUPERVISOR (Turn {}) ---", turns);
            
            let mut grounding = String::new();
            
            // Tool Independence: Only scrape if we don't have enough history or if it's a browser-heavy task
            let has_adequate_history = plan_history.contains("Result:") || plan_history.contains("gathered information");
            
            if !has_adequate_history || turns > 1 {
                if let Some(dom) = get_browser_dom_tree().await {
                    grounding.push_str(&format!("--- BROWSER DOM DATA ---\n{}\n\n", dom));
                }
                
                let res = perception_tool.call(PerceptionArgs {
                    action: "get_screen_text".to_string(), app: Some("active".to_string())
                }).await;
                grounding.push_str(&res.unwrap_or_else(|_| "No perception data available.".to_string()));
            } else {
                grounding.push_str("Using research data from PLAN_HISTORY. No live screen scraping performed for turn 1.");
            }
            
            let json_candidate = self.step(goal, &grounding, turns, plan_history).await?;
            println!("{}", json_candidate);
            
            let mut x = None; let mut y = None; let mut t = None; let mut ty = String::new(); let mut v = false;
            let mut b = None; let mut c = None;
            if let Some(output) = execute_tools(&json_candidate, "", perception_tool, &mut x, &mut y, &mut t, &mut ty, &mut v, &mut b, &mut c, false).await {
                println!("--- TOOL OUTPUT ---\n{}", output);
                if output.contains("TASK_COMPLETED") || output.to_lowercase().contains("code executed successfully") {
                    println!("✅ TEXT WRITER: Summary written successfully.");
                    return Ok("Text Writer task completed successfully.".to_string());
                }
            }
        }
        Err("TextWriter execution limit reached.".into())
    }
}
