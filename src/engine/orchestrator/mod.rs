mod dispatcher;
pub mod dynamic;
pub mod master;
mod planner;

pub use dynamic::{ChatAgent, OverseerAgent, VisionGeneralAgent};
pub use master::MasterPlanner;
pub use planner::MacroTask;

use rig::{
    client::CompletionClient,
    completion::{Message, Prompt},
    providers::ollama,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

fn user_facing_runtime_error(details: &str) -> Option<String> {
    let lower = details.to_lowercase();

    let ollama_related = lower.contains("localhost:11434")
        || lower.contains("/api/chat")
        || lower.contains("/api/generate")
        || lower.contains("/api/embed")
        || lower.contains("ollama");

    let connection_related = lower.contains("error sending request")
        || lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("http client error")
        || lower.contains("tcp connect error")
        || lower.contains("dns error");

    if ollama_related || connection_related {
        return Some(
            "Model backend unavailable. Overseer could not reach Ollama on `http://localhost:11434`.\n\n\
             Try this:\n\
             1. Start Ollama with `ollama serve`\n\
             2. Check it responds with `ollama list`\n\
             3. Pull the required models, for example `ollama pull qwen2.5:7b-instruct`\n\n\
             After Ollama is healthy, send the message again."
                .to_string(),
        );
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrchestrationRoute {
    DirectChat,
    SingleDomain,
    MultiStep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestrationDecision {
    route: OrchestrationRoute,
    domain: Option<String>,
    personality_mode: String,
    reason: String,
}

fn infer_domain_from_query(query: &str) -> Option<String> {
    let lower = query.to_lowercase();

    if lower.contains("research")
        || lower.contains("search")
        || lower.contains("browse")
        || lower.contains("website")
        || lower.contains("web")
    {
        Some("stealth".to_string())
    } else if lower.contains("browser")
        || lower.contains("open site")
        || lower.contains("open website")
        || lower.contains("click")
    {
        Some("web".to_string())
    } else if lower.contains("desktop")
        || lower.contains("app")
        || lower.contains("window")
        || lower.contains("ui")
        || lower.contains("folder")
        || lower.contains("file")
    {
        Some("desktop".to_string())
    } else if lower.contains("document")
        || lower.contains("pdf")
        || lower.contains("spreadsheet")
        || lower.contains("writer")
        || lower.contains("calc")
        || lower.contains("office")
    {
        Some("office".to_string())
    } else if lower.contains("image")
        || lower.contains("vision")
        || lower.contains("screenshot")
        || lower.contains("photo")
    {
        Some("vision".to_string())
    } else if lower.contains("music") || lower.contains("audio") || lower.contains("neontune") {
        Some("neontune".to_string())
    } else {
        None
    }
}

fn heuristic_decision(query: &str) -> OrchestrationDecision {
    let lower = query.to_lowercase();
    let direct_chat_markers = [
        "what",
        "who",
        "when",
        "where",
        "why",
        "how",
        "explain",
        "tell me",
        "help me understand",
        "summarize",
    ];
    let delegation_markers = [
        "open ",
        "launch ",
        "click",
        "create",
        "generate",
        "research",
        "search",
        "browse",
        "download",
        "analyze",
        "compare",
        "workflow",
        "step by step",
        "automation",
    ];

    let looks_like_direct_chat = query.len() < 220
        && direct_chat_markers.iter().any(|marker| lower.contains(marker))
        && !delegation_markers
            .iter()
            .any(|marker| lower.contains(marker));

    if looks_like_direct_chat {
        return OrchestrationDecision {
            route: OrchestrationRoute::DirectChat,
            domain: Some("chat".to_string()),
            personality_mode: "assistant".to_string(),
            reason: "Short informational request with no clear automation or tool need.".to_string(),
        };
    }

    if let Some(domain) = infer_domain_from_query(query) {
        return OrchestrationDecision {
            route: OrchestrationRoute::SingleDomain,
            domain: Some(domain.clone()),
            personality_mode: match domain.as_str() {
                "stealth" => "analyst".to_string(),
                "web" | "desktop" | "office" | "neontune" => "operator".to_string(),
                "vision" => "vision".to_string(),
                _ => "specialist".to_string(),
            },
            reason: "Request maps cleanly to a single operational domain.".to_string(),
        };
    }

    OrchestrationDecision {
        route: OrchestrationRoute::MultiStep,
        domain: None,
        personality_mode: "orchestrator".to_string(),
        reason: "Request appears to require orchestration or multi-step delegation.".to_string(),
    }
}

async fn decide_orchestration(
    all_agents: &[AgentInfo],
    query: &str,
    memory_context: &str,
    ollama_client: &ollama::Client,
) -> Result<OrchestrationDecision, String> {
    let heuristic = heuristic_decision(query);

    if matches!(heuristic.route, OrchestrationRoute::DirectChat) {
        return Ok(heuristic);
    }

    let Some(orchestrator_agent) = all_agents.iter().find(|a| a.domain == "orchestrator") else {
        return Ok(heuristic);
    };

    let decision_agent =
        rig::agent::AgentBuilder::new(ollama_client.completion_model(&orchestrator_agent.model))
            .preamble(&format!(
                "{}\n\nYou are the routing control layer for Overseer.\n\
                 Decide how the request should be handled.\n\
                 Return JSON only with this shape:\n\
                 {{\"route\":\"direct_chat|single_domain|multi_step\",\"domain\":\"chat|web|stealth|desktop|office|vision|neontune|null\",\"personality_mode\":\"assistant|analyst|operator|vision|orchestrator\",\"reason\":\"short explanation\"}}\n\
                 Rules:\n\
                 - direct_chat: use when the request can be answered conversationally without tools or execution.\n\
                 - single_domain: use when one domain should handle the request.\n\
                 - multi_step: use when the request needs planning across steps or domains.\n\
                 - Prefer direct_chat for simple Q&A.\n\
                 - Prefer single_domain when one supervisor should own execution.\n\
                 - Use multi_step for compound or ambiguous tasks.",
                orchestrator_agent.instructions
            ))
            .default_max_turns(1)
            .build();

    let prompt = format!(
        "USER_REQUEST:\n{}\n\nRECENT_CONTEXT:\n{}\n\nReturn only JSON.",
        query,
        if memory_context.is_empty() {
            "No recent context.".to_string()
        } else {
            memory_context.to_string()
        }
    );

    let raw = decision_agent
        .prompt(&prompt)
        .await
        .map_err(|e| e.to_string())?;
    let cleaned = crate::engine::utils::clean_response(raw);

    serde_json::from_str::<OrchestrationDecision>(&cleaned)
        .or_else(|_| {
            serde_json::from_str::<serde_json::Value>(&cleaned).map(|value| {
                let route = match value
                    .get("route")
                    .and_then(|v| v.as_str())
                    .unwrap_or("multi_step")
                {
                    "direct_chat" => OrchestrationRoute::DirectChat,
                    "single_domain" => OrchestrationRoute::SingleDomain,
                    _ => OrchestrationRoute::MultiStep,
                };

                OrchestrationDecision {
                    route,
                    domain: value
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .filter(|value| !value.trim().is_empty() && *value != "null")
                        .map(|value| value.to_string()),
                    personality_mode: value
                        .get("personality_mode")
                        .and_then(|v| v.as_str())
                        .unwrap_or("orchestrator")
                        .to_string(),
                    reason: value
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Decision returned by orchestrator classifier.")
                        .to_string(),
                }
            })
        })
        .or_else(|_| Ok(heuristic))
}

async fn record_event(
    db: &crate::db::Database,
    run_id: Option<i64>,
    sequence: &mut i64,
    event_type: &str,
    status: &str,
    domain: Option<&str>,
    step_text: Option<&str>,
    tool_name: Option<&str>,
    model: Option<&str>,
    message: Option<&str>,
    details: Option<&str>,
) {
    if let Some(run_id) = run_id {
        let current_sequence = *sequence;
        *sequence += 1;
        if let Err(e) = crate::backend::log_execution_event(
            db,
            run_id,
            current_sequence,
            event_type,
            status,
            domain,
            step_text,
            tool_name,
            model,
            message,
            details,
        )
        .await
        {
            println!("⚠️  Failed to log execution event '{}': {}", event_type, e);
        }
    }
}

pub async fn orchestrate_request<'a>(
    db: &crate::db::Database,
    conversation_id: i64,
    master_planner: &'a MasterPlanner,
    all_agents: &[AgentInfo],
    query: &str,
    history: Vec<Message>,
    perception_tool: &StructuredPerceptionTool,
    memory_context: &str,
    ollama_client: &ollama::Client,
) -> String {
    let model_timeout_ms =
        crate::backend::get_setting_i64(db, "model_timeout_ms", 90_000).await as u64;
    let run_id = crate::backend::start_execution_run(db, conversation_id, query)
        .await
        .ok();
    let mut event_sequence = 1_i64;

    record_event(
        db,
        run_id,
        &mut event_sequence,
        "run_started",
        "running",
        None,
        None,
        None,
        None,
        Some("Execution run started."),
        None,
    )
    .await;

    let decision = match decide_orchestration(all_agents, query, memory_context, ollama_client).await
    {
        Ok(decision) => decision,
        Err(err) => {
            if let Some(friendly_error) = user_facing_runtime_error(&err) {
                record_event(
                    db,
                    run_id,
                    &mut event_sequence,
                    "model_backend_unavailable",
                    "error",
                    Some("orchestrator"),
                    None,
                    None,
                    None,
                    Some("Model backend unavailable during orchestration decision."),
                    Some(&err),
                )
                .await;
                if let Some(run_id) = run_id {
                    let _ = crate::backend::finish_execution_run(
                        db,
                        run_id,
                        "failed",
                        None,
                        Some(&friendly_error),
                    )
                    .await;
                }
                return friendly_error;
            }
            heuristic_decision(query)
        }
    };

    let decision_details = serde_json::to_string(&decision).unwrap_or_default();
    record_event(
        db,
        run_id,
        &mut event_sequence,
        "orchestration_decision",
        "completed",
        decision.domain.as_deref(),
        None,
        None,
        None,
        Some(&format!(
            "Orchestrator chose {:?} with personality '{}'.",
            decision.route, decision.personality_mode
        )),
        Some(&decision_details),
    )
    .await;

    if matches!(decision.route, OrchestrationRoute::DirectChat) {
        let chat_info = all_agents
            .iter()
            .find(|a| a.domain == "chat")
            .cloned()
            .unwrap_or_else(|| AgentInfo {
                id: 0,
                name: "Chat Assistant".to_string(),
                description: "Direct chat fallback".to_string(),
                tools: Vec::new(),
                instructions: "You are a concise, helpful assistant. Answer directly and clearly without using tools unless explicitly required.".to_string(),
                model: "qwen2.5:7b-instruct".to_string(),
                domain: "chat".to_string(),
                version: "1.0.0".to_string(),
                code: None,
                source_files: Vec::new(),
            });

        let chat_agent = ChatAgent::new(ollama_client, &chat_info.model, &chat_info.instructions);
        let direct_context = if memory_context.is_empty() {
            "No recent context.".to_string()
        } else {
            memory_context.to_string()
        };

        record_event(
            db,
            run_id,
            &mut event_sequence,
            "direct_chat_started",
            "running",
            Some("chat"),
            Some(query),
            None,
            Some(&chat_info.model),
            Some("Routing request directly to chat agent."),
            Some(&decision_details),
        )
        .await;

        let direct_response = chat_agent.answer(query, &direct_context).await;
        if let Some(friendly_error) = user_facing_runtime_error(&direct_response) {
            record_event(
                db,
                run_id,
                &mut event_sequence,
                "direct_chat_failed",
                "error",
                Some("chat"),
                Some(query),
                None,
                Some(&chat_info.model),
                Some("Direct chat failed because the model backend is unavailable."),
                Some(&direct_response),
            )
            .await;
            if let Some(run_id) = run_id {
                let _ = crate::backend::finish_execution_run(
                    db,
                    run_id,
                    "failed",
                    None,
                    Some(&friendly_error),
                )
                .await;
            }
            return friendly_error;
        }

        record_event(
            db,
            run_id,
            &mut event_sequence,
            "direct_chat_completed",
            "completed",
            Some("chat"),
            Some(query),
            None,
            Some(&chat_info.model),
            Some("Direct chat completed."),
            None,
        )
        .await;

        if let Some(run_id) = run_id {
            let _ = crate::backend::finish_execution_run(
                db,
                run_id,
                "completed",
                Some(&direct_response),
                None,
            )
            .await;
        }

        return direct_response;
    }

    let macro_tasks = if matches!(decision.route, OrchestrationRoute::SingleDomain) {
        let domain = decision
            .domain
            .clone()
            .unwrap_or_else(|| "chat".to_string());
        let synthetic_plan = serde_json::json!([{
            "domain": domain,
            "task": query,
        }])
        .to_string();
        record_event(
            db,
            run_id,
            &mut event_sequence,
            "macro_plan_generated",
            "completed",
            decision.domain.as_deref(),
            Some(query),
            None,
            None,
            Some("Orchestrator routed request directly to a single domain."),
            Some(&synthetic_plan),
        )
        .await;
        planner::sanitize_plan(&synthetic_plan, query)
    } else {
        println!("🧠 MASTER PLANNER Thinking...");
        let plan_text: String = match timeout(
            Duration::from_millis(model_timeout_ms),
            master_planner.plan(query),
        )
        .await
        {
            Ok(plan) => plan,
            Err(_) => {
                let message = format!("Master planner timed out after {} ms.", model_timeout_ms);
                record_event(
                    db,
                    run_id,
                    &mut event_sequence,
                    "planner_timeout",
                    "error",
                    Some("orchestrator"),
                    None,
                    None,
                    None,
                    Some(&message),
                    None,
                )
                .await;
                if let Some(run_id) = run_id {
                    let _ = crate::backend::finish_execution_run(
                        db,
                        run_id,
                        "failed",
                        None,
                        Some(&message),
                    )
                    .await;
                }
                return format!("PLAN EXECUTION HALTED.\nError: {}", message);
            }
        };
        println!("📋 MASTER PLAN RAW: {}", plan_text);

        if let Some(friendly_error) = user_facing_runtime_error(&plan_text) {
            record_event(
                db,
                run_id,
                &mut event_sequence,
                "model_backend_unavailable",
                "error",
                Some("orchestrator"),
                None,
                None,
                None,
                Some("Model backend unavailable during master planning."),
                Some(&plan_text),
            )
            .await;
            if let Some(run_id) = run_id {
                let _ = crate::backend::finish_execution_run(
                    db,
                    run_id,
                    "failed",
                    None,
                    Some(&friendly_error),
                )
                .await;
            }
            return friendly_error;
        }

        record_event(
            db,
            run_id,
            &mut event_sequence,
            "macro_plan_generated",
            "completed",
            Some("orchestrator"),
            None,
            None,
            None,
            Some("Master planner generated a macro plan."),
            Some(&plan_text),
        )
        .await;

        planner::sanitize_plan(&plan_text, query)
    };

    let mut final_report = String::new();
    let mut browser_pid: Option<i32> = None;

    let safe_memory = memory_context
        .replace("TASK_COMPLETED", "[PAST RUN FINISHED]")
        .replace("completed successfully", "[past success]")
        .replace("PLAN EXECUTION HALTED", "[INTERRUPTED]")
        .replace("Error:", "Status:")
        .replace("CompletionError:", "NetworkStatus:")
        .replace("ResponseError:", "GenerationStatus:");

    let memories = if safe_memory.is_empty() {
        "No relevant past experiences.".to_string()
    } else {
        format!(
            "--- HISTORICAL REFERENCE DATA ---\n{}\n--- END HISTORICAL DATA ---\n\n\
             WARNING: The records above are from PREVIOUS runs. They may contain old coordinates or statuses. \
             Use them ONLY for coordinate reference if the UI hasn't changed. Do NOT assume current progress.",
            safe_memory
        )
    };

    let mut plan_history = "Plan execution just started.".to_string();
    let mut bypass_result: Option<String> = None;

    for (m_idx, macro_task) in macro_tasks.iter().enumerate() {
        println!("\n=======================================================");
        println!(
            "🚀 MACRO TASK {}/{}: [{}] {}",
            m_idx + 1,
            macro_tasks.len(),
            macro_task.domain.to_uppercase(),
            macro_task.task
        );
        println!("=======================================================");

        record_event(
            db,
            run_id,
            &mut event_sequence,
            "macro_task_started",
            "running",
            Some(&macro_task.domain),
            Some(&macro_task.task),
            None,
            None,
            Some("Starting macro task."),
            None,
        )
        .await;

        let planner_domain = format!("{}_planner", macro_task.domain);

        let db_planner = sqlx::query_as::<_, crate::db::Planner>(
            "SELECT id, name, description, instructions, model, domain, version, code FROM planners WHERE domain = ? OR domain = ?"
        )
        .bind(&macro_task.domain)
        .bind(&planner_domain)
        .fetch_optional(&db.pool)
        .await
        .ok()
        .flatten();

        let micro_steps: Vec<String> = if let Some(p) = db_planner {
            println!("💡 DYNAMIC PLANNER: Using '{}' configuration.", p.name);
            let model_name = p
                .model
                .clone()
                .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
            let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model_name))
                .preamble(&p.instructions.clone().unwrap_or_default())
                .default_max_turns(1)
                .build();

            let sub_plan = match timeout(
                Duration::from_millis(model_timeout_ms),
                agent.prompt(&macro_task.task),
            )
            .await
            {
                Ok(Ok(plan)) => plan,
                Ok(Err(e)) => format!("[\"Error planning: {}\"]", e),
                Err(_) => format!("[\"Planner timed out after {} ms\"]", model_timeout_ms),
            };

            record_event(
                db,
                run_id,
                &mut event_sequence,
                "micro_plan_generated",
                "completed",
                Some(&macro_task.domain),
                Some(&macro_task.task),
                None,
                Some(&model_name),
                Some("Domain planner generated micro-steps."),
                Some(&sub_plan),
            )
            .await;

            if let Some(friendly_error) = user_facing_runtime_error(&sub_plan) {
                record_event(
                    db,
                    run_id,
                    &mut event_sequence,
                    "model_backend_unavailable",
                    "error",
                    Some(&macro_task.domain),
                    Some(&macro_task.task),
                    None,
                    Some(&model_name),
                    Some("Model backend unavailable during domain planning."),
                    Some(&sub_plan),
                )
                .await;
                if let Some(run_id) = run_id {
                    let _ = crate::backend::finish_execution_run(
                        db,
                        run_id,
                        "failed",
                        None,
                        Some(&friendly_error),
                    )
                    .await;
                }
                return friendly_error;
            }

            planner::parse_micro_steps(&sub_plan, &macro_task.task)
        } else if let Some(agent_info) = all_agents.iter().find(|a| {
            a.domain == planner_domain
                || a.domain == macro_task.domain
                    && (a.name.to_lowercase().contains("planner") || a.domain == "orchestrator")
        }) {
            let model_name = if agent_info.model.is_empty() {
                "qwen2.5:7b-instruct".to_string()
            } else {
                agent_info.model.clone()
            };
            let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model_name))
                .preamble(&agent_info.instructions)
                .default_max_turns(1)
                .build();

            let sub_plan = match timeout(
                Duration::from_millis(model_timeout_ms),
                agent.prompt(&macro_task.task),
            )
            .await
            {
                Ok(Ok(plan)) => plan,
                Ok(Err(e)) => format!("[\"Error planning: {}\"]", e),
                Err(_) => format!("[\"Planner timed out after {} ms\"]", model_timeout_ms),
            };

            record_event(
                db,
                run_id,
                &mut event_sequence,
                "micro_plan_generated",
                "completed",
                Some(&macro_task.domain),
                Some(&macro_task.task),
                None,
                Some(&model_name),
                Some("Fallback planner generated micro-steps."),
                Some(&sub_plan),
            )
            .await;

            if let Some(friendly_error) = user_facing_runtime_error(&sub_plan) {
                record_event(
                    db,
                    run_id,
                    &mut event_sequence,
                    "model_backend_unavailable",
                    "error",
                    Some(&macro_task.domain),
                    Some(&macro_task.task),
                    None,
                    Some(&model_name),
                    Some("Model backend unavailable during fallback planning."),
                    Some(&sub_plan),
                )
                .await;
                if let Some(run_id) = run_id {
                    let _ = crate::backend::finish_execution_run(
                        db,
                        run_id,
                        "failed",
                        None,
                        Some(&friendly_error),
                    )
                    .await;
                }
                return friendly_error;
            }

            planner::parse_micro_steps(&sub_plan, &macro_task.task)
        } else {
            let fallback_steps = match macro_task.domain.as_str() {
                "math" | "text_writer" | "chat" | "stealth" => vec![macro_task.task.clone()],
                _ => {
                    println!(
                        "⚠️ No specific planner found for domain '{}', using task directly.",
                        macro_task.domain
                    );
                    vec![macro_task.task.clone()]
                }
            };

            record_event(
                db,
                run_id,
                &mut event_sequence,
                "micro_plan_fallback",
                "completed",
                Some(&macro_task.domain),
                Some(&macro_task.task),
                None,
                None,
                Some("No planner found; using task directly."),
                None,
            )
            .await;

            fallback_steps
        };

        for (i, step) in micro_steps.iter().enumerate() {
            println!(
                "\n▶️ EXECUTING MICRO-STEP {}/{}...",
                i + 1,
                micro_steps.len()
            );

            record_event(
                db,
                run_id,
                &mut event_sequence,
                "micro_step_started",
                "running",
                Some(&macro_task.domain),
                Some(step),
                None,
                None,
                Some("Executing micro-step."),
                None,
            )
            .await;

            let step_lower = step.to_lowercase();

            if step_lower.starts_with("wait") || step_lower.contains("wait for") {
                println!("⏱️ AUTO-WAIT: Pausing 2s...");
                sleep(Duration::from_secs(2)).await;
                let result = format!("Wait step completed: '{}'", step);
                plan_history.push_str(&format!(
                    "\nStep {}: '{}' -> Result: {}",
                    i + 1,
                    step,
                    result
                ));
                final_report.push_str(&format!("\n\n### Step {}: {}\n{}", i + 1, step, result));
                record_event(
                    db,
                    run_id,
                    &mut event_sequence,
                    "micro_step_completed",
                    "completed",
                    Some(&macro_task.domain),
                    Some(step),
                    None,
                    None,
                    Some(&result),
                    None,
                )
                .await;
                continue;
            }

            let open_app = if step_lower.starts_with("open ") {
                Some(
                    step_lower
                        .trim_start_matches("open ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            } else if step_lower.starts_with("launch ") {
                Some(
                    step_lower
                        .trim_start_matches("launch ")
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            };

            if let Some(ref app_name) = open_app {
                let is_pure_open = !step_lower.contains(" and ")
                    && !step_lower.contains(", ")
                    && !step_lower.contains(" then ");
                let stop_words = [
                    "the", "a", "an", "repo", "link", "his", "go", "to", "open", "launch",
                    "github", "website",
                ];
                if is_pure_open && !app_name.is_empty() && !stop_words.contains(&app_name.as_str())
                {
                    let browser_apps = ["chrome", "firefox", "browser", "chromium", "web"];
                    if browser_apps.iter().any(|b| app_name.contains(b)) {
                        println!(
                            "🌐 AUTO-BROWSER-OPEN: Starting Playwright daemon for '{}'...",
                            app_name
                        );
                        let result =
                            format!("Auto-opened browser for {}. (Placeholder logic)", app_name);
                        plan_history.push_str(&format!(
                            "\nStep {}: '{}' -> Result: {}",
                            i + 1,
                            step,
                            result
                        ));
                        final_report.push_str(&format!(
                            "\n\n### Step {}: {}\n{}",
                            i + 1,
                            step,
                            result
                        ));
                        record_event(
                            db,
                            run_id,
                            &mut event_sequence,
                            "micro_step_completed",
                            "completed",
                            Some(&macro_task.domain),
                            Some(step),
                            None,
                            None,
                            Some(&result),
                            None,
                        )
                        .await;
                        continue;
                    }
                }
            }

            match dispatcher::dispatch_domain(
                db,
                run_id,
                &mut event_sequence,
                all_agents,
                &macro_task.domain,
                step,
                query,
                history.clone(),
                &memories,
                &plan_history,
                memory_context,
                &macro_task.task,
                perception_tool,
                ollama_client,
                model_timeout_ms,
                &mut browser_pid,
            )
            .await
            {
                Ok(r) => {
                    let mut supervisor_result = r.clone();
                    let mut final_result = if supervisor_result.raw_response.starts_with("---CHART_SVG---")
                        || supervisor_result.raw_response.starts_with("---CHART_HTML---")
                        || supervisor_result.raw_response.starts_with("---CHART_ASSET---")
                    {
                        supervisor_result.raw_response.clone()
                    } else {
                        supervisor_result.display_text()
                    };

                    let interactive_domains = ["web", "desktop", "neontune", "app"];
                    if supervisor_result.status == "completed"
                        && interactive_domains.contains(&macro_task.domain.as_str())
                    {
                        println!(
                            "🔍 GROUNDING: Verifying success state for domain '{}'...",
                            macro_task.domain
                        );

                        let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
                        if let Some(paddle_tool) =
                            tools_list.iter().find(|t| t.name == "paddle_ui_automation")
                        {
                            if let Ok(state_json) = crate::engine::tools::call_dynamic_tool(
                                paddle_tool,
                                serde_json::json!({"input": "get_state"}),
                            )
                            .await
                            {
                                let grounding_agent =
                                    all_agents.iter().find(|a| a.name == "Grounding Agent");
                                if let Some(ga) = grounding_agent {
                                    let ga_client = rig::agent::AgentBuilder::new(
                                        ollama_client.completion_model(&ga.model),
                                    )
                                    .preamble(&ga.instructions)
                                    .build();

                                    let ga_prompt = format!(
                                        "GOAL: {}\nTASK: {}\n\nVISIBLE UI TEXT:\n{}\n\nHas the task been successfully completed? Respond with STATE_VERIFIED or STATE_MISMATCH: [reason]",
                                        query, step, state_json
                                    );

                                    if let Ok(Ok(ga_response)) = timeout(
                                        Duration::from_millis(model_timeout_ms),
                                        ga_client.prompt(&ga_prompt),
                                    )
                                    .await
                                    {
                                        println!("👁️ GROUNDING EYE: {}", ga_response);
                                        if ga_response.contains("STATE_MISMATCH") {
                                            supervisor_result =
                                                dispatcher::SupervisorExecutionResult {
                                                    status: "failed".to_string(),
                                                    summary: "Grounding verification failed."
                                                        .to_string(),
                                                    evidence: Vec::new(),
                                                    artifacts: Vec::new(),
                                                    verified: false,
                                                    error: Some(ga_response.clone()),
                                                    raw_response: ga_response.clone(),
                                                };
                                            final_result = supervisor_result.display_text();
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let event_status = match supervisor_result.status.as_str() {
                        "failed" => "error",
                        _ if supervisor_result.verified => "verified",
                        _ => "unverified",
                    };
                    let supervisor_details = serde_json::to_string(&supervisor_result).ok();

                    if supervisor_result.status == "failed" {
                        record_event(
                            db,
                            run_id,
                            &mut event_sequence,
                            "micro_step_failed",
                            event_status,
                            Some(&macro_task.domain),
                            Some(step),
                            None,
                            None,
                            Some(&final_result),
                            supervisor_details.as_deref(),
                        )
                        .await;
                        if let Some(run_id) = run_id {
                            let _ = crate::backend::finish_execution_run(
                                db,
                                run_id,
                                "failed",
                                None,
                                supervisor_result.error.as_deref().or(Some(&final_result)),
                            )
                            .await;
                        }
                        return format!(
                            "PLAN EXECUTION HALTED.\nError: {}\nLast State: {}",
                            final_result, final_report
                        );
                    }

                    let supervisor_bypasses = sqlx::query_scalar::<_, bool>(
                        "SELECT bypass_synthesizer FROM supervisors WHERE domain = ? LIMIT 1",
                    )
                    .bind(&macro_task.domain)
                    .fetch_optional(&db.pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(false);

                    if supervisor_bypasses {
                        println!("⚡ BYPASS: domain '{}' has bypass_synthesizer=true — stashing result, will skip synthesizer.", macro_task.domain);
                        bypass_result = Some(final_result.clone());
                    }

                    plan_history.push_str(&format!(
                        "\nStep {}: '{}' -> Result: {}",
                        i + 1,
                        step,
                        final_result
                    ));
                    final_report.push_str(&format!(
                        "\n\n### Step {}: {}\n{}",
                        i + 1,
                        step,
                        final_result
                    ));
                    record_event(
                        db,
                        run_id,
                        &mut event_sequence,
                        "micro_step_completed",
                        event_status,
                        Some(&macro_task.domain),
                        Some(step),
                        None,
                        None,
                        Some(&final_result),
                        supervisor_details.as_deref(),
                    )
                    .await;
                }
                Err(e) => {
                    record_event(
                        db,
                        run_id,
                        &mut event_sequence,
                        "micro_step_failed",
                        "error",
                        Some(&macro_task.domain),
                        Some(step),
                        None,
                        None,
                        Some(&e),
                        None,
                    )
                    .await;
                    if let Some(run_id) = run_id {
                        let _ = crate::backend::finish_execution_run(
                            db,
                            run_id,
                            "failed",
                            None,
                            Some(&e),
                        )
                        .await;
                    }
                    return format!(
                        "PLAN EXECUTION HALTED.\nError: {}\nLast State: {}",
                        e, final_report
                    );
                }
            }
        }
    }

    if let Some(bypass) = bypass_result {
        println!("⚡ BYPASS SYNTHESIZER: returning bypass result directly.");
        if let Some(run_id) = run_id {
            let _ =
                crate::backend::finish_execution_run(db, run_id, "completed", Some(&bypass), None)
                    .await;
        }
        return bypass;
    }

    let chart_markers = ["---CHART_SVG---", "---CHART_HTML---", "---CHART_ASSET---"];
    for marker in &chart_markers {
        if let Some(pos) = final_report.find(marker) {
            println!("📊 CHART MARKER found in final_report — bypassing synthesizer.");
            let chart_result = final_report[pos..].trim().to_string();
            if let Some(run_id) = run_id {
                let _ = crate::backend::finish_execution_run(
                    db,
                    run_id,
                    "completed",
                    Some(&chart_result),
                    None,
                )
                .await;
            }
            return chart_result;
        }
    }

    let orchestrator_agent = all_agents.iter().find(|a| a.domain == "orchestrator");
    let synthesizer_agent = all_agents.iter().find(|a| a.domain == "synthesizer");

    let synthesis_model = synthesizer_agent
        .map(|a| a.model.as_str())
        .unwrap_or_else(|| {
            orchestrator_agent
                .map(|a| a.model.as_str())
                .unwrap_or("qwen2.5:7b-instruct")
        });

    let synthesis_instructions = synthesizer_agent
        .map(|a| a.instructions.as_str())
        .unwrap_or("You are a helpful assistant. Present research findings clearly in plain Markdown. Never output JSON.");

    let synthesis_agent =
        rig::agent::AgentBuilder::new(ollama_client.completion_model(synthesis_model))
            .preamble(synthesis_instructions)
            .build();

    let synthesis_prompt = format!(
        "USER_QUERY: {}\n\nRESEARCH FINDINGS:\n{}\n\n\
         Instructions: Provide a clear response to the user. \
         If the findings show a simple success, keep it extremely brief (one sentence). \
         Only be detailed if the research findings are complex.",
        query, final_report
    );

    record_event(
        db,
        run_id,
        &mut event_sequence,
        "synthesis_started",
        "running",
        Some("orchestrator"),
        None,
        None,
        Some(synthesis_model),
        Some("Synthesizing final response."),
        None,
    )
    .await;

    let final_response = match timeout(
        Duration::from_millis(model_timeout_ms),
        synthesis_agent.prompt(&synthesis_prompt),
    )
    .await
    {
        Ok(Ok(synthesis)) => synthesis,
        Ok(Err(_)) | Err(_) => final_report.clone(),
    };

    record_event(
        db,
        run_id,
        &mut event_sequence,
        "run_completed",
        "completed",
        Some("orchestrator"),
        None,
        None,
        Some(synthesis_model),
        Some("Execution run completed."),
        None,
    )
    .await;

    if let Some(run_id) = run_id {
        let _ = crate::backend::finish_execution_run(
            db,
            run_id,
            "completed",
            Some(&final_response),
            None,
        )
        .await;
    }

    final_response
}
