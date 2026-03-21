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
                    .map(|t| format!("- {}: {}", t.name, t.description))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "AVAILABLE TOOLS:\n{}\n\n\
                     To use a tool, respond ONLY with JSON: {{\"tool\": \"tool_name\", \"input\": {{...}}}}\n\
                     OPTIONAL CHARTING: Use the charting tool ONLY if a visualization is explicitly requested or if you are comparing 3 or more data points. \n\
                     When you call charting, its result (starting with ---CHART_SVG---) MUST be your COMPLETE final response.\n\
                     Do NOT add any text, markdown, or summary before or after a chart marker.\n\n",
                    list
                )
            };

            // Check if this is a chat domain with charting tools available
            let has_charting_tool = tools.iter().any(|t| t.name == "charting");

            let clean_context = sanitize_context(memory_context);

            let clean_plan = sanitize_context(plan_history);

            let initial_prompt = format!(
                "GOAL: {}\n\
                 TASK: {}\n\n\
                 {}CONTEXT:\n{}\n\n\
                 PROGRESS:\n{}\n\n\
                 COMMAND: Execute the task using tools if necessary. Provide your final findings.",
                query, step, tool_descriptions, clean_context, clean_plan
            );

            println!("📝 PROMPT TEXT:\n{}", initial_prompt);
            println!("📜 PREAMBLE:\n{}", s.instructions.as_deref().unwrap_or(""));

            // Fetch max_iterations from settings
            let max_iterations_str = crate::backend::get_setting(db, "max_iterations").await;
            let max_iterations = max_iterations_str.parse::<usize>().unwrap_or(10);

            let mut current_prompt = initial_prompt;
            let mut final_response = String::new();
            let mut manual_svg_retries = 0;
            const MAX_SVG_RETRIES: usize = 2;

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
                println!("🔍 RAW MODEL RESPONSE (first 500 chars): {}", &trimmed[..trimmed.len().min(500)]);

                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(tool_name) = json_val.get("tool").and_then(|v| v.as_str()) {
                        let input_val = json_val.get("input").unwrap_or(&serde_json::Value::Null);
                        println!("🛠️ MODEL CALLED TOOL: '{}' | raw input type: {}", tool_name,
                            if input_val.is_string() { "string (will try to parse as JSON)" }
                            else if input_val.is_object() { "object" }
                            else { "other" }
                        );

                        let input_json = if input_val.is_string() {
                            let s = input_val.as_str().unwrap();
                            println!("📋 RAW INPUT STRING: {}", s);
                            match serde_json::from_str::<serde_json::Value>(s) {
                                Ok(parsed) => {
                                    println!("✅ Input string parsed as JSON: {}", parsed);
                                    parsed
                                }
                                Err(e) => {
                                    println!("⚠️ Input string is NOT valid JSON ({}), wrapping as {{input: s}}", e);
                                    serde_json::json!({"input": s})
                                }
                            }
                        } else {
                            println!("📋 INPUT OBJECT: {}", input_val);
                            input_val.clone()
                        };

                        if tool_name == "charting" {
                            println!("📊 CHARTING FINAL INPUT TO TOOL: {}", input_json);
                            
                            // 🔒 Intent Guardrail: ONLY allow charting if the user explicitly asked for a visualization
                            let viz_keywords = ["chart", "graph", "plot", "visualize", "visualization", "draw", "sketch", "diagram", "pie", "bar", "mapped"];
                            let user_query_lower = query.to_lowercase();
                            let explicitly_requested = viz_keywords.iter().any(|&k| user_query_lower.contains(k));
                            
                            if !explicitly_requested {
                                println!("🚫 INTENT GUARDRAIL: Model tried to chart but user did not use visualization keywords. Forcing plain text.");
                                let next_instruction = "Next: Provide your answer in PLAIN TEXT. Comparison should be a list or table, NOT a chart, unless the user explicitly asks for one.";
                                current_prompt = format!(
                                    "{}\n\n[SYSTEM OVERRIDE: The user did not ask for a chart or graph. Provide your final answer in PLAIN TEXT ONLY. Do NOT use the charting tool for this request.]\n\n{}",
                                    current_prompt, next_instruction
                                );
                                continue;
                            }
                        }

                        let tool_info = tools.iter().find(|t| t.name == tool_name);

                        let tool_result = match tool_info {
                            Some(info) => {
                                crate::engine::tools::call_dynamic_tool(info, input_json)
                                    .await
                                    .unwrap_or_else(|e| format!("Tool error: {}", e))
                            }
                            None => format!("Tool '{}' not found.", tool_name),
                        };

                        if tool_name == "charting" {
                            println!("📦 CHARTING TOOL RESULT (first 200): {}", &tool_result[..tool_result.len().min(200)]);
                            println!("📦 CHARTING RESULT STARTS WITH MARKER: {}",
                                tool_result.starts_with("---CHART_SVG---") || tool_result.starts_with("---CHART_HTML---"));
                        } else {
                            println!("📦 TOOL RESULT: {}", &tool_result[..tool_result.len().min(300)]);
                        }

                        if tool_name == "charting" && (
                            tool_result.starts_with("---CHART_SVG---") ||
                            tool_result.starts_with("---CHART_HTML---")
                        ) {
                            println!("✅ Chart generated — returning directly without model passthrough");
                            final_response = tool_result;
                            break;
                        }

                        if tool_name == "charting" {
                            println!("❌ CHARTING TOOL DID NOT RETURN A CHART MARKER — result was: {}", &tool_result[..tool_result.len().min(500)]);
                        }

                        // For non-chart tools, continue the loop as normal
                        let next_instruction = "Next: Call another tool (JSON) or provide your final answer.";
                        current_prompt = format!(
                            "{}\n\n[RESULT for '{}' ({}):\n{}]\n\n{}",
                            current_prompt, tool_name, input_val, tool_result, next_instruction
                        );
                        continue;
                    }
                } else {
                    println!("🔍 NOT A TOOL CALL — treating as final answer candidate");
                }

                // Not a tool call — check for manual SVG abuse on chat domain
                if domain == "chat" && has_charting_tool && contains_manual_visualization(&response) {
                    manual_svg_retries += 1;
                    println!("⚠️ Agent wrote manual SVG/HTML (retry {}/{}). Forcing charting tool call.", manual_svg_retries, MAX_SVG_RETRIES);

                    if manual_svg_retries <= MAX_SVG_RETRIES {
                        // Extract any data values from the response to help the retry
                        let retry_instruction = format!(
                            "{}\n\n[SYSTEM OVERRIDE: Your previous response contained manual SVG/HTML code. \
                            This is FORBIDDEN. You MUST call the charting tool instead. \
                            Do NOT write any SVG, HTML, or visualization code. \
                            Extract the data values from your previous response and call the charting tool now: \
                            {{\"tool\": \"charting\", \"input\": {{\"chart_type\": \"pie|bar|line\", \
                            \"title\": \"your title\", \"labels\": [\"2024\", \"2025\", \"2026\"], \
                            \"data\": [value1, value2, value3]}}}}]\n\n\
                            Call the charting tool NOW with the correct data.",
                            current_prompt
                        );
                        current_prompt = retry_instruction;
                        continue;
                    } else {
                        // Max retries exceeded — give up and return error
                        println!("❌ Max SVG retries exceeded. Agent refuses to use charting tool.");
                        final_response = "Chart generation failed — model wrote manual SVG instead of using the charting tool. Please try again.".to_string();
                        break;
                    }
                }

                // Valid final response
                println!("✅ FINAL RESPONSE received at iteration {}", iteration + 1);
                final_response = post_process_for_domain(domain, response);
                break;
            }

            if final_response.is_empty() {
                final_response = "Max tool iterations reached without a final response.".to_string();
            }

            Ok(final_response)
        }
    }
}

/// Check if a response contains manual visualization code that should have used the charting tool
fn contains_manual_visualization(response: &str) -> bool {
    let lower = response.to_lowercase();
    // Raw SVG without proper chart marker
    if lower.contains("<svg") && !lower.contains("---chart_svg---") {
        return true;
    }
    // Fenced code blocks with visual content
    if lower.contains("```html") 
        || lower.contains("```svg") 
        || lower.contains("```xml") 
    {
        return true;
    }
    // Comment-style SVG markers the agent uses as fake markers
    if lower.contains("<!-- svg") || lower.contains("<!-- chart") {
        return true;
    }
    false
}

fn post_process_for_domain(domain: &str, response: String) -> String {
    match domain {
        "stealth" => clean_stealth_output(response),
        _ => post_process_agent_response(response),
    }
}

fn clean_stealth_output(response: String) -> String {
    response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Skip markdown headers
            if trimmed.starts_with('#') { return None; }
            // Skip code fence lines
            if trimmed.starts_with("```") { return None; }
            // Skip pure separator lines
            if trimmed == "---" || trimmed == "***" { return None; }
            // Strip inline markdown: bold, italic, bullet points
            let cleaned = trimmed
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .trim_start_matches("• ");
            let cleaned = cleaned
                .replace("**", "")
                .replace("__", "")
                .replace('*', "")
                .replace('_', "");
            if cleaned.trim().is_empty() { return None; }
            Some(cleaned)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn post_process_agent_response(response: String) -> String {
    // Safety net: if a chart marker somehow ended up in a text response,
    // return only the text before it — the chart will come in its own turn.
    if let Some(pos) = response.find("---CHART_SVG---")
        .or_else(|| response.find("---CHART_HTML---"))
    {
        let before = response[..pos].trim().to_string();
        if before.is_empty() {
            // Pure chart — return as-is
            return response[pos..].to_string();
        }
        // Mixed response: return only the text, discard the embedded chart
        return before;
    }
    response
}

fn sanitize_context(context: &str) -> String {
    let svg_tags = ["<rect ", "<path ", "<polyline ", "<text ", "<svg", "</svg>", 
                    "<style", "<defs", "<clipPath", "data:image/svg"];
    context
        .lines()
        .filter(|line| {
            let lower = line.trim().to_lowercase();
            !svg_tags.iter().any(|tag| lower.contains(tag))
                && !lower.starts_with("```svg")
                && !lower.starts_with("```html")
                && lower != "```"
                && !lower.contains("![")
        })
        .collect::<Vec<_>>()
        .join("\n")
}