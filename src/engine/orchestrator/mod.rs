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
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::AgentInfo;

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

    let macro_tasks = planner::sanitize_plan(&plan_text, query);

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
                    let mut final_result = r.clone();

                    let interactive_domains = ["web", "desktop", "neontune", "app"];
                    if (r.trim().to_lowercase() == "done" || r.to_lowercase().contains("success"))
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
                                            final_result =
                                                format!("GROUNDING ERROR: {}", ga_response);
                                        }
                                    }
                                }
                            }
                        }
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
                        "completed",
                        Some(&macro_task.domain),
                        Some(step),
                        None,
                        None,
                        Some(&final_result),
                        None,
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
