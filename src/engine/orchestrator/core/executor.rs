use crate::db::Database;
use crate::engine::orchestrator::core::{event_logger::record_event, response::StructuredResponse, response::MessagePart};
use crate::engine::orchestrator::planning::planner::MacroTask;
use crate::engine::utils::discovery::AgentInfo;
use crate::engine::orchestrator::dispatch;
use crate::engine::tools::StructuredPerceptionTool;
use rig::completion::{Message, CompletionModel, Prompt};
use rig::client::CompletionClient;
use rig::providers::ollama;
use crate::engine::orchestrator::utils::context_helpers::{format_newly_created_artifacts, format_step_results_context};
use crate::engine::orchestrator::core::state::StructuredState;

pub async fn execute_macro_tasks(
    db: &Database,
    conversation_id: i64,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    all_agents: &[AgentInfo],
    macro_tasks: Vec<MacroTask>,
    clean_query: &str,
    history: &[Message],
    memories: &str,
    perception_tool: &StructuredPerceptionTool,
    ollama_client: &ollama::Client,
    model_timeout_ms: u64,
) -> String {
    let mut master_response = StructuredResponse::new();
    let mut plan_history = String::new();
    let mut active_memories = memories.to_string();
    let mut structured_state = StructuredState::new();
    let mut browser_pid: Option<i32> = None;

    for (task_idx, macro_task) in macro_tasks.iter().enumerate() {
        println!(
            "\n🚀 EXECUTING MACRO TASK {}/{}: [{}] {}...",
            task_idx + 1,
            macro_tasks.len(),
            macro_task.domain,
            macro_task.task
        );

        record_event(
            db,
            run_id,
            event_sequence,
            "macro_task_started",
            "running",
            Some(&macro_task.domain),
            Some(&macro_task.task),
            None,
            None,
            Some("Executing macro task."),
            None,
        )
        .await;

        let micro_steps = vec![macro_task.task.clone()]; // Simplified for restoration

        for (i, step) in micro_steps.iter().enumerate() {
            println!("\n▶️ EXECUTING MICRO-STEP {}/{}...", i + 1, micro_steps.len());

            record_event(
                db,
                run_id,
                event_sequence,
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

            let mut retry_count = 0;
            let max_retries = 2;
            let mut current_retry_msg: Option<String> = None;
            let mut final_result: Option<crate::engine::orchestrator::core::result::SupervisorExecutionResult> = None;
            let mut current_domain = macro_task.domain.clone();

            while retry_count <= max_retries {
                match dispatch::dispatch_domain(
                    db,
                    run_id,
                    event_sequence,
                    all_agents,
                    &current_domain,
                    step,
                    clean_query,
                    history.to_vec(),
                    &plan_history,
                    &structured_state,
                    ollama_client,
                    model_timeout_ms,
                    current_retry_msg.as_deref(),
                )
                .await
                {
                    Ok(r) => {
                        let mut result_to_check = r;
                        // ARCHITECTURE V3: CLAIM COMPILER
                        // Instead of a post-hoc verifier, we 'compile' the response by matching claims to Evidence IDs.
                        if let Some(compiled_result) = compile_response(
                            db,
                            run_id,
                            event_sequence,
                            step,
                            &result_to_check,
                            &structured_state,
                            ollama_client,
                        ).await {
                             if compiled_result.status == "stalled" {
                                println!("⚠️ COMPILER detected grounding failure or ABSTENTION.");
                                result_to_check = compiled_result;
                             } else {
                                result_to_check = compiled_result;
                             }
                        }
                        
                        // STALL DETECTION: If the verifier or worker says it is stalled, we retry
                        if result_to_check.status == "stalled" && retry_count < max_retries {
                            println!("⚠️ STALL DETECTED in domain '{}'. Triggering autonomous retry {}/{}...", current_domain, retry_count + 1, max_retries);
                            
                            // ARCHITECTURE V2.7: CROSS-DOMAIN RECOVERY (Artifact Hand-off)
                            // If we stall twice on an artifact task in the wrong domain, pivot to Office.
                            let lower_task = step.to_lowercase();
                            let is_artifact_task = ["pdf", "docx", "report", "csv", "xlsx", "chart"].iter().any(|&kw| lower_task.contains(kw));
                            if retry_count == 1 && is_artifact_task && current_domain != "office" {
                                println!("🛡️ ARTIFACT HAND-OFF: Stalled domain '{}' lacks office tools. Pivoting to 'office' for final attempt.", current_domain);
                                current_domain = "office".to_string();
                            }

                            current_retry_msg = if retry_count == 0 {
                                Some("Your previous response was rejected because you were planning instead of acting OR claimed success without calling a tool. CALL A TOOL NOW. Do not describe future intent.".to_string())
                            } else {
                                // ARCHITECTURE V2: SILICON MANDATE (Terminal Escalation)
                                let domain_instruction = if current_domain == "office" {
                                    "YOU ARE NOW IN THE OFFICE DOMAIN. FORBIDDEN: Do NOT search. MANDATE: Use 'document_writer' to generate the requested file using ONLY the context provided."
                                } else {
                                    "1. IF YOU LACK DATA, USE 'stealth_search' OR 'web_search'.\n2. IF THE TASK IS TO GENERATE, USE 'document_writer'."
                                };

                                Some(format!("═══ TERMINAL ESCALATION (SILICON MANDATE) ═══\n\
                                      YOU HAVE REPEATEDLY FAILED TO ACT. THIS IS YOUR FINAL ATTEMPT.\n\
                                      1. YOU ARE FORBIDDEN FROM RETURNING 'status: completed' WITHOUT A TOOL CALL.\n\
                                      2. YOU MUST IMMEDIATELY CALL A PHYSICAL TOOL (JSON).\n\
                                      {}\n\
                                      3. ANY RESPONSE WITHOUT A TOOL CALL WILL BE AUTOMATICALLY REJECTED.", domain_instruction))
                            };
                            
                            retry_count += 1;
                            
                            record_event(
                                db,
                                run_id,
                                event_sequence,
                                "stall_detected",
                                "retrying",
                                Some(&current_domain),
                                Some(step),
                                None,
                                None,
                                Some("Stall detected: agent is planning instead of acting. Retrying with terminal Silicon Mandate."),
                                None,
                            ).await;
                            continue;
                        }

                        final_result = Some(result_to_check);
                        // If we succeeded after a retry, we should remove the previous error message for this step
                        // from the master_response to avoid user confusion.
                        if retry_count > 0 {
                            let step_error_prefix = format!("❌ ERROR: Task failed: ");
                            master_response.messages.retain(|m| !m.message.contains(step) || !m.message.starts_with(&step_error_prefix));
                            // Also clear any previous "Tool missing" or hallucination errors
                            master_response.messages.retain(|m| !m.message.contains("Tool") && !m.message.contains("missing"));
                            // Also remove "STALLED" or "ABSTAIN" messages for this step if they were added
                            master_response.messages.retain(|m| !m.message.contains("ABSTAIN") || !m.message.contains(step));
                        }
                        break;
                    }
                    Err(e) => {
                        let error_msg = format!("Task failed: {}", e);
                        master_response.messages.push(MessagePart {
                            message: format!("❌ ERROR: {}", error_msg),
                            role: "assistant".to_string(),
                        });
                        record_event(
                            db,
                            run_id,
                            event_sequence,
                            "micro_step_failed",
                            "error",
                            Some(&macro_task.domain),
                            Some(step),
                            None,
                            None,
                            Some(&error_msg),
                            None,
                        )
                        .await;
                        break;
                    }
                }
            }

            if let Some(res) = final_result {
                let mut merged_res = res.clone();
                // ... logic to handle merging if needed, though final_result is already verified/merged
                
                let step_structured = crate::engine::orchestrator::utils::formatter::format_supervisor_result(&merged_res);
                master_response.merge(step_structured);
                
                // --- ARCHITECTURE V3: CONTRADICTION GUARD ---
                let conflict = crate::engine::orchestrator::core::consistency::check_contradictions(
                    ollama_client,
                    &structured_state,
                    &merged_res.summary
                ).await;

                if conflict.has_conflict {
                    let reason = conflict.reason.unwrap_or_else(|| "Unknown logical conflict".to_string());
                    println!("☢️  CONTRADICTION DETECTED! Reason: {}", reason);
                    record_event(
                        db,
                        run_id,
                        event_sequence,
                        "contradiction_detected",
                        "stalled",
                        Some("consistency_guard"),
                        Some(step),
                        None,
                        None,
                        Some(&reason),
                        None,
                    ).await;
                    
                    // Force a stall to trigger a retry or research branch
                    merged_res.status = "stalled".to_string();
                    merged_res.summary = format!("ABSTAIN | REASON: Logical contradiction found with existing evidence. Conflict: {}", reason);
                }

                if !merged_res.artifacts.is_empty() {
                    let new_ctx = format_newly_created_artifacts(&merged_res.artifacts, step);
                    active_memories.push_str("\n\n");
                    active_memories.push_str(&new_ctx);
                }

                // --- ARCHITECTURE V3: STATE UPDATE ---
                let clean_summary = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(&merged_res.summary);
                structured_state.update_from_findings(&merged_res.findings, &clean_summary, &merged_res.evidence);
                
                // The 'active_memories' passed to the next step is now the structured state
                active_memories = structured_state.to_prompt_string();

                let display = merged_res.display_text();
                plan_history.push_str(&format!("\nStep {}: '{}' -> Result: {}", i + 1, step, display));

                // Clean grounding markers [E1], [E2] etc from the master response message
                if let Some(msg) = master_response.messages.last_mut() {
                    if msg.role == "assistant" {
                        msg.message = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(&msg.message);
                    }
                }

                record_event(
                    db,
                    run_id,
                    event_sequence,
                    "micro_step_completed",
                    "completed",
                    Some(&macro_task.domain),
                    Some(step),
                    None,
                    None,
                    Some(&display),
                    None,
                )
                .await;
            }
        }
    }

    let final_report = serde_json::to_string(&master_response).unwrap_or_else(|_| "{\"status\":\"failed\"}".to_string());
    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, "completed", Some(&final_report), None).await;
    }

    final_report
}

/// Architecture V3: Claim Compiler
/// Instead of a generic verification, this 'compiles' the response by matching claims to Evidence IDs.
/// Unsupported claims are stripped silently. If too many claims are ungrounded, the turn is discarded.
async fn compile_response(
    db: &Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    step_description: &str,
    result: &crate::engine::orchestrator::core::result::SupervisorExecutionResult,
    state: &StructuredState,
    ollama_client: &ollama::Client,
) -> Option<crate::engine::orchestrator::core::result::SupervisorExecutionResult> {
    println!("🛡️ COMPILING RESPONSE for grounding enforcement...");
    
    // 1. PRE-UPDATE: Create a preview state including THIS turn's findings
    let preview_state = state.preview_update(&result.findings, &result.summary, &result.evidence);
    let state_prompt = preview_state.to_prompt_string();

    // 2. EXTRACTION PHASE: Identify claims and their cited evidence IDs
    let extraction_prompt = format!(
        "WORKER_SUMMARY: {}\n\n\
         {}\n\n\
         ═══ COMPILATION PROTOCOL ═══\n\
         1. EXTRACT CLAIMS: Break the summary into individual factual sentences.\n\
         2. MAP CITATIONS: For each sentence, identify the `[E#]` ID cited by the worker.\n\
         3. FORMAT: Respond ONLY with a JSON list of objects: `[{{\"sentence\": \"...\", \"cited_id\": \"E1\"}}, ...]`. If no citation exists, use `null` for `cited_id`.\n\n\
         INSTRUCTION: Extract all claims and citations. Do NOT add commentary.",
        result.summary, state_prompt
    );

    let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model("llama3.2:3b"))
        .preamble("Extract factual claims and citations from the worker summary as JSON.")
        .build();
    let raw_extraction = match agent.prompt(&extraction_prompt).await {
        Ok(resp) => resp,
        Err(_) => return None,
    };

    let claims: Vec<serde_json::Value> = if let Some(start) = raw_extraction.find('[') {
        if let Some(end) = raw_extraction.rfind(']') {
            serde_json::from_str(&raw_extraction[start..=end]).unwrap_or_default()
        } else { vec![] }
    } else { vec![] };

    if claims.is_empty() {
        return None;
    }

    // 3. MATCHING PHASE: Verify citations against StructuredState
    let mut grounded_sentences = Vec::new();
    let mut total_claims = claims.len();
    let mut matching_claims = 0;

    for claim_val in claims {
        let sentence = claim_val["sentence"].as_str().unwrap_or_default();
        let cited_id = claim_val["cited_id"].as_str();

        if let Some(id) = cited_id {
            if preview_state.evidence.iter().any(|e| e.id == id) {
                grounded_sentences.push(sentence.to_string());
                matching_claims += 1;
            } else {
                println!("⚠️  SILENCING Sentence (Invalid ID {}): {}", id, sentence);
            }
        } else {
            println!("⚠️  SILENCING Sentence (No Citation): {}", sentence);
        }
    }

    // 4. ABSTENTION PHASE: Discard turn if confidence is low
    let grounding_score = matching_claims as f32 / total_claims as f32;
    if grounding_score < 0.5 {
        println!("❌ ABSTENTION TRIGGERED: Grounding score {:.2} below threshold. Discarding turn.", grounding_score);
        record_event(
            db,
            run_id,
            event_sequence,
            "abstention_triggered",
            "stalled",
            Some("compiler"),
            Some(step_description),
            None,
            None,
            Some(&format!("Abstention: Only {}/{} claims were grounded.", matching_claims, total_claims)),
            None,
        ).await;

        let mut stalled_res = result.clone();
        stalled_res.status = "stalled".to_string();
        stalled_res.summary = "ABSTAIN | REASON: INSUFFICIENT_EVIDENCE to ground the response.".to_string();
        return Some(stalled_res);
    }

    let mut final_res = result.clone();
    final_res.summary = grounded_sentences.join(" ");
    final_res.verified = true;

    record_event(
        db,
        run_id,
        event_sequence,
        "compilation_complete",
        "completed",
        Some("compiler"),
        Some(step_description),
        None,
        None,
        Some(&format!("Compiled response. Kept {}/{} sentences.", matching_claims, total_claims)),
        None,
    ).await;

    Some(final_res)
}
