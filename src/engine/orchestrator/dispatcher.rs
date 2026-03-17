use rig::providers::ollama;
use crate::engine::orchestrator::OverseerAgent;
use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;
use rig::agent::AgentBuilder;
use rig::completion::{Message, Prompt};
use rig::client::CompletionClient;

pub async fn dispatch_domain(
    db: &crate::db::Database,
    agent_registry: &[AgentInfo],
    domain: &str,
    step: &str,
    query: &str,
    _history: Vec<Message>,
    _memories: &str,
    plan_history: &str,
    memory_context: &str,
    _macro_task_description: &str,
    _perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    _browser_pid: &mut Option<i32>,
) -> Result<String, String> {
    // 1. Resolve the supervisor for this domain from the DB
    let supervisor = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, code FROM supervisors WHERE domain = ?"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await
    .map_err(|e| format!("Database error resolving supervisor: {}", e))?;

    // 2. Resolve the agents associated with this supervisor
    let _linked_agents = if let Some(ref s) = supervisor {
        crate::backend::get_supervisor_agents(db, s.id).await
            .map_err(|e| format!("Error fetching supervisor agents: {}", e))?
    } else {
        vec![]
    };

    // 3. Helper to find agent info or fallback to registry/defaults
    let get_agent = |domain_name: &str| {
        agent_registry.iter().find(|a| a.domain == domain_name)
            .cloned()
            .unwrap_or_else(|| AgentInfo {
                id: 0,
                name: domain_name.to_string(),
                description: String::new(),
                tools: Vec::new(),
                instructions: String::new(),
                model: "qwen2.5:7b-instruct".to_string(),
                domain: domain_name.to_string(),
                version: "1.0.0".to_string(),
                code: None,
                source_files: Vec::new(),
            })
    };

    // Instantiate overseer (needed by most supervisors)
    let overseer_info = get_agent("overseer");
    let _overseer = OverseerAgent::new(ollama_client, &overseer_info.model, &overseer_info.instructions);

    // 4. Dispatch
    match domain {
        _ => {
            println!("🤖 DISPATCH: Dynamic supervisor for domain '{}'", domain);
            let s = supervisor.clone().ok_or_else(|| format!("No supervisor found in DB for domain '{}'.", domain))?;
            println!("🤖 DISPATCH: Dynamic supervisor '{}' using model '{:?}'", s.name, s.model);

            // Fetch tools
            let tools = if let Some(ref s) = supervisor {
                crate::backend::get_supervisor_tools(db, s.id).await.unwrap_or_default()
            } else {
                vec![]
            };

            // Build agent WITHOUT Rig's tool system — we handle tool calling manually
            let agent = AgentBuilder::new(
                ollama_client.completion_model(s.model.as_deref().unwrap_or("qwen2.5:7b-instruct"))
            )
            .preamble(s.instructions.as_deref().unwrap_or(""))
            .build();

            // Format tool schemas into the prompt
            let tool_descriptions = if tools.is_empty() {
                String::new()
            } else {
                let list = tools.iter()
                    .map(|t| format!(
                        "- {}: {}\n  Call with: {{\"tool\": \"{}\", \"input\": \"your input\"}}",
                        t.name, t.description, t.name
                    ))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "AVAILABLE TOOLS:\n{}\n\n\
                     To use a tool, respond ONLY with JSON: {{\"tool\": \"tool_name\", \"input\": \"your input\"}}\n\
                     After seeing the tool result, provide your final answer as plain text.\n\n",
                    list
                )
            };

            let initial_prompt = format!(
                "GOAL: {}\n\
                 TASK: {}\n\n\
                 {}CONTEXT:\n{}\n\n\
                 PROGRESS:\n{}\n\n\
                 COMMAND: Execute the task using tools if necessary. Provide your final findings.",
                query, step, tool_descriptions, memory_context, plan_history
            );

            println!("📝 PROMPT TEXT:\n{}", initial_prompt);
            println!("📜 PREAMBLE:\n{}", s.instructions.as_deref().unwrap_or(""));

            // Agentic loop — handle tool calls manually
            let mut current_prompt = initial_prompt;
            let mut final_response = String::new();
            let max_iterations = 5;

            for iteration in 0..max_iterations {
                println!("🔄 AGENT ITERATION {}/{}", iteration + 1, max_iterations);

                let response = agent.prompt(&current_prompt)
                    .await
                    .map_err(|e| {
                        eprintln!("❌ ERROR calling dynamic agent: {}", e);
                        format!("Dynamic supervisor error for domain '{}': {}", domain, e)
                    })?;

                // Check if the response is a tool call
                let trimmed = response.trim();
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let (Some(tool_name), Some(input)) = (
                        json_val.get("tool").and_then(|v| v.as_str()),
                        json_val.get("input").and_then(|v| v.as_str()),
                    ) {
                        println!("🛠️ MODEL CALLED TOOL: '{}' with input: '{}'", tool_name, input);

                        let tool_info = tools.iter().find(|t| t.name == tool_name);
                        let tool_result = match tool_info {
                            Some(info) => {
                                crate::engine::tools::call_dynamic_tool(
                                    info,
                                    serde_json::json!({"input": input})
                                )
                                .await
                                .unwrap_or_else(|e| format!("Tool error: {}", e))
                            }
                            None => format!(
                                "Tool '{}' not found. Available tools: {}",
                                tool_name,
                                tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", ")
                            ),
                        };

                        println!("📦 TOOL RESULT: {}", &tool_result[..tool_result.len().min(300)]);

                        // Append result and continue loop
                        current_prompt = format!(
                            "{}\n\nTOOL RESULT for '{}' ({}):\n{}\n\n\
                             Next: Call another tool using JSON, or provide your final answer (e.g., 'done').",
                            current_prompt, tool_name, input, tool_result
                        );
                        continue;
                    }
                }

                // Not a tool call — treat as final answer
                println!("✅ FINAL RESPONSE received at iteration {}", iteration + 1);
                final_response = response;
                break;
            }

            if final_response.is_empty() {
                final_response = "Max tool iterations reached without a final response.".to_string();
            }

            Ok(final_response)
        }
    }
}