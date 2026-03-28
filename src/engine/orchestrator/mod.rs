pub mod discovery;
mod dispatcher;
pub mod dynamic;
pub mod grounding;
pub mod guardrails;
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

fn should_bypass_final_synthesizer(domains: &[String]) -> bool {
    !domains.is_empty()
        && domains
            .iter()
            .all(|domain| matches!(domain.as_str(), "computer" | "desktop"))
}

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
             3. Pull at least one model, then select it in Settings if needed, for example `ollama pull qwen3:8b`\n\n\
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

fn infer_native_computer_domain(query: &str) -> Option<String> {
    let lower = query.to_lowercase();

    let screenshot_request = (lower.contains("screenshot")
        || lower.contains("screeshot")
        || lower.contains("screen shot")
        || lower.contains("screen capture")
        || lower.contains("capture screen")
        || lower.contains("capture my "))
        && (lower.contains("desktop")
            || lower.contains("screen")
            || lower.contains("workspace")
            || lower.contains("window")
            || lower.contains("app")
            || lower.contains("pc")
            || lower.contains("computer"));

    if screenshot_request {
        return Some("computer".to_string());
    }

    let launch_request = (lower.contains("open ")
        || lower.contains("launch ")
        || lower.contains("start "))
        && (lower.contains("pc")
            || lower.contains("computer")
            || lower.contains("app")
            || lower.contains("discord")
            || lower.contains("chrome")
            || lower.contains("browser")
            || lower.contains("vscode")
            || lower.contains("visual studio code")
            || lower.contains("spotify")
            || lower.contains("explorer"));

    if launch_request {
        return Some("computer".to_string());
    }

    let accessibility_request = lower.contains("accessibility")
        || lower.contains("ui automation")
        || lower.contains("automation tree")
        || lower.contains("focused element")
        || lower.contains("focused control")
        || lower.contains("inspect ui")
        || lower.contains("inspect accessibility");
    if accessibility_request {
        return Some("computer".to_string());
    }

    let selector_control_request = (lower.contains("focus element")
        || lower.contains("focus control")
        || lower.contains("invoke ")
        || lower.contains("invoke element")
        || lower.contains("invoke control")
        || lower.contains("click element")
        || lower.contains("press button"))
        && (lower.contains("button")
            || lower.contains("field")
            || lower.contains("input")
            || lower.contains("control")
            || lower.contains("element")
            || lower.contains("window")
            || lower.contains("selector")
            || lower.contains("automation id")
            || lower.contains("save")
            || lower.contains("ok")
            || lower.contains("submit"));
    if selector_control_request {
        return Some("computer".to_string());
    }

    None
}

fn is_native_computer_fast_path_request(query: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }

    if infer_native_computer_domain(query).is_some() {
        return true;
    }

    let lower = query.to_lowercase();

    let process_request = lower.contains("background task")
        || lower.contains("background tasks")
        || lower.contains("running process")
        || lower.contains("running processes")
        || lower.contains("process list")
        || lower.contains("active process")
        || lower.contains("active processes")
        || lower.contains("what's running")
        || lower.contains("whats running")
        || ((lower.contains("currently running")
            || lower.contains("current running")
            || lower.contains("what is currently running")
            || lower.contains("what is running")
            || lower.contains("show what is running")
            || lower.contains("tell me what is running"))
            && (lower.contains("computer")
                || lower.contains("pc")
                || lower.contains("machine")
                || lower.contains("system")));

    let file_search_request = (lower.contains("resume")
        || lower.contains("file path")
        || lower.contains("find file")
        || lower.contains("locate file")
        || lower.contains(".pdf")
        || lower.contains(".doc")
        || lower.contains(".docx")
        || ((lower.contains("find")
            || lower.contains("locate")
            || lower.contains("where is"))
            && (lower.contains("file") || lower.contains("path"))))
        && !lower.contains("open file explorer");

    let browser_state_request = lower.contains("current tab")
        || lower.contains("current page")
        || lower.contains("browser state")
        || lower.contains("which browser")
        || (lower.contains("browser") && lower.contains("open"));

    let selector_typing_request = (lower.contains("type ") || lower.contains("enter "))
        && (lower.contains(" into ") || lower.contains(" in "))
        && (lower.contains("field")
            || lower.contains("input")
            || lower.contains("textbox")
            || lower.contains("text box")
            || lower.contains("search box")
            || lower.contains("element")
            || lower.contains("control"));

    process_request || file_search_request || browser_state_request || selector_typing_request
}

async fn heuristic_decision(
    db: &crate::db::Database,
    query: &str,
    has_artifacts: bool,
) -> OrchestrationDecision {
    let lower = query.to_lowercase();
    // Phase 2: Orchestration Generalization
    let inferred_domain = discovery::discover_fast_path_domain(db, query, has_artifacts).await;
    let direct_chat_markers = [
        "hello",
        "hi",
        "hey",
        "help",
        "tell me a joke",
        "who are you",
        "what can you do",
    ];
    let delegation_markers = [
        "open ",
        "launch ",
        "click",
        "create",
        "generate",
        "research",
        "search",
        "find",
        "get",
        "retrieve",
        "list",
        "check",
        "report",
        "browse",
        "download",
        "analyze",
        "compare",
        "workflow",
        "step by step",
        "automation",
        "observe",
        "inspect",
        "watch",
        "active window",
        "current window",
        "foreground window",
        "background task",
        "background tasks",
        "running process",
        "running processes",
        "what's running",
        "whats running",
        "resume path",
        "find file",
        "locate file",
        "file path",
        "where is",
        "current tab",
        "current page",
        "browser state",
        "which browser",
    ];

    let looks_like_direct_chat = query.len() < 220
        && inferred_domain.is_none()
        && direct_chat_markers
            .iter()
            .any(|marker| lower.contains(marker))
        && !delegation_markers
            .iter()
            .any(|marker| lower.contains(marker));

    if looks_like_direct_chat {
        return OrchestrationDecision {
            route: OrchestrationRoute::DirectChat,
            domain: Some("chat".to_string()),
            personality_mode: "assistant".to_string(),
            reason: "Short informational request with no clear automation or tool need."
                .to_string(),
        };
    }

    if let Some(domain) = inferred_domain {
        return OrchestrationDecision {
            route: OrchestrationRoute::SingleDomain,
            domain: Some(domain.clone()),
            personality_mode: match domain.as_str() {
                "stealth" => "analyst".to_string(),
                "web" | "desktop" | "computer" | "office" | "neontune" => "operator".to_string(),
                "vision" => "vision".to_string(),
                _ => "specialist".to_string(),
            },
            reason: "Request maps cleanly to a single operational domain.".to_string(),
        };
    }

    if let Some(domain) = infer_native_computer_domain(query) {
        return OrchestrationDecision {
            route: OrchestrationRoute::SingleDomain,
            domain: Some(domain),
            personality_mode: "operator".to_string(),
            reason: "Request maps to a native computer action.".to_string(),
        };
    }

    OrchestrationDecision {
        route: OrchestrationRoute::MultiStep,
        domain: None,
        personality_mode: "orchestrator".to_string(),
        reason: "Request appears to require orchestration or multi-step delegation.".to_string(),
    }
}

fn is_affirmative_follow_up(query: &str) -> bool {
    let normalized = query.trim().to_lowercase();
    matches!(
        normalized.as_str(),
        "continue"
            | "yes"
            | "go ahead"
            | "proceed"
            | "continue."
            | "yes."
            | "okay continue"
            | "ok continue"
            | "ok"
            | "okay"
            | "do it"
            | "yes please"
    ) || normalized.starts_with("continue ")
        || normalized.starts_with("go ahead")
        || normalized.starts_with("yes,")
        || normalized.starts_with("proceed ")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScreenshotFollowUpIntent {
    Locate,
    Resend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArtifactReference {
    id: i64,
    label: String,
    kind: String,
    domain: Option<String>,
    file_path: Option<String>,
    content_type: Option<String>,
    content_excerpt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkItemReference {
    id: i64,
    title: String,
    brief: Option<String>,
    status: String,
    surface_kind: String,
    assigned_domain: Option<String>,
    prompt: String,
}

fn extract_structured_context(
    query: &str,
) -> (String, Vec<ArtifactReference>, Vec<WorkItemReference>) {
    let mut refs = Vec::new();
    let mut work_items = Vec::new();
    let mut visible_lines = Vec::new();

    for line in query.lines() {
        let trimmed = line.trim();
        if trimmed == "ARTIFACT_CONTEXT:" || trimmed == "WORK_ITEM_CONTEXT:" {
            continue;
        }
        if let Some(payload) = trimmed
            .strip_prefix("[[ARTIFACT_REF:")
            .and_then(|value| value.strip_suffix("]]"))
        {
            if let Ok(parsed) = serde_json::from_str::<ArtifactReference>(payload) {
                refs.push(parsed);
            }
            continue;
        }
        if let Some(payload) = trimmed
            .strip_prefix("[[WORK_ITEM_REF:")
            .and_then(|value| value.strip_suffix("]]"))
        {
            if let Ok(parsed) = serde_json::from_str::<WorkItemReference>(payload) {
                work_items.push(parsed);
            }
            continue;
        }
        visible_lines.push(line);
    }

    (
        visible_lines.join("\n").trim().to_string(),
        refs,
        work_items,
    )
}

fn format_artifact_context(artifact_refs: &[ArtifactReference]) -> String {
    if artifact_refs.is_empty() {
        return String::new();
    }

    let lines = artifact_refs
        .iter()
        .map(|artifact| {
            let mut parts = vec![
                format!("Artifact #{}", artifact.id),
                format!("label={}", artifact.label),
                format!("kind={}", artifact.kind),
            ];
            if let Some(domain) = artifact
                .domain
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("domain={domain}"));
            }
            if let Some(path) = artifact
                .file_path
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("path={path}"));
            }
            if let Some(content_type) = artifact
                .content_type
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("content_type={content_type}"));
            }
            if let Some(excerpt) = artifact
                .content_excerpt
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("excerpt={excerpt}"));
            }
            format!("- {}", parts.join(" | "))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("SELECTED ARTIFACTS:\n{}", lines)
}

fn format_work_item_context(work_items: &[WorkItemReference]) -> String {
    if work_items.is_empty() {
        return String::new();
    }

    let lines = work_items
        .iter()
        .map(|work_item| {
            let mut parts = vec![
                format!("WorkItem #{}", work_item.id),
                format!("title={}", work_item.title),
                format!("status={}", work_item.status),
                format!("surface={}", work_item.surface_kind),
                format!("prompt={}", work_item.prompt),
            ];
            if let Some(domain) = work_item
                .assigned_domain
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("assigned_domain={domain}"));
            }
            if let Some(brief) = work_item
                .brief
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                parts.push(format!("brief={brief}"));
            }
            format!("- {}", parts.join(" | "))
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("ACTIVE WORK ITEMS:\n{}", lines)
}

fn classify_screenshot_follow_up(query: &str) -> Option<ScreenshotFollowUpIntent> {
    let normalized = query
        .trim()
        .to_lowercase()
        .replace('?', "")
        .replace('.', "");
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    let locate_markers = [
        "where is it",
        "where is the screenshot",
        "where did you save it",
        "where was it saved",
        "what is the path",
        "show me the path",
        "where is that image",
        "where is the image",
    ];
    if locate_markers
        .iter()
        .any(|marker| compact == *marker || compact.contains(marker))
    {
        return Some(ScreenshotFollowUpIntent::Locate);
    }

    let resend_markers = [
        "send it here",
        "show it here",
        "attach it here",
        "send it",
        "show it",
        "attach it",
        "send the screenshot here",
        "show the screenshot here",
        "put it here",
        "display it here",
    ];
    if resend_markers
        .iter()
        .any(|marker| compact == *marker || compact.contains(marker))
    {
        return Some(ScreenshotFollowUpIntent::Resend);
    }

    None
}

async fn latest_assistant_image_attachment(
    db: &crate::db::Database,
    conversation_id: i64,
) -> Option<crate::backend::Attachment> {
    let messages = crate::backend::get_messages(db, conversation_id)
        .await
        .ok()?;
    for message in messages.into_iter().rev() {
        if message.role != "assistant" {
            continue;
        }
        if let Some(attachment) = message
            .attachments
            .into_iter()
            .rev()
            .find(|attachment| attachment.content_type.starts_with("image/"))
        {
            return Some(attachment);
        }
    }
    None
}

async fn resolve_screenshot_follow_up(
    db: &crate::db::Database,
    conversation_id: i64,
    query: &str,
) -> Option<(String, String)> {
    let intent = classify_screenshot_follow_up(query)?;
    let attachment = latest_assistant_image_attachment(db, conversation_id).await?;
    let path = attachment.file_path.trim();
    if path.is_empty() {
        return None;
    }

    let response = match intent {
        ScreenshotFollowUpIntent::Locate => format!(
            "The latest desktop screenshot is saved here:\n\n{}\n\nFilename: {}",
            path, attachment.filename
        ),
        ScreenshotFollowUpIntent::Resend => format!(
            "Here is the latest desktop screenshot.\n\n[[ATTACH_IMAGE:{}]]",
            path
        ),
    };

    let details = serde_json::json!({
        "intent": match intent {
            ScreenshotFollowUpIntent::Locate => "locate",
            ScreenshotFollowUpIntent::Resend => "resend",
        },
        "path": path,
        "filename": attachment.filename,
    })
    .to_string();

    Some((response, details))
}

async fn decide_orchestration(
    db: &crate::db::Database,
    all_agents: &[AgentInfo],
    query: &str,
    history: &[Message],
    memory_context: &str,
    ollama_client: &ollama::Client,
    has_artifacts: bool,
) -> Result<OrchestrationDecision, String> {
    // 1. Check for Fast-Path keyword matches from the database (fully dynamic)
    if let Some(domain) =
        crate::engine::orchestrator::discovery::discover_fast_path_domain(db, query, has_artifacts)
            .await
    {
        return Ok(OrchestrationDecision {
            route: OrchestrationRoute::SingleDomain,
            domain: Some(domain.clone()),
            personality_mode: "operator".to_string(),
            reason: format!("Fast-path routing match found for domain: {}", domain),
        });
    }

    // 2. Handle affirmative follow-ups (e.g., "yes", "ok", "continue")
    if is_affirmative_follow_up(query) {
        // Look for the original intent in history to understand the goal
        let original_intent = history
            .iter()
            .find(|m| matches!(m, Message::User { .. }))
            .map(|m| match m {
                Message::User { content, .. } => serde_json::to_string(content).unwrap_or_default(),
                _ => String::new(),
            })
            .unwrap_or_else(|| query.to_string());

        // Dynamic Supervisor Discovery: Iterate all supervisors and check their tool guardrails
        let all_supervisors = crate::backend::get_supervisors(db)
            .await
            .unwrap_or_default();

        // Priority-ordered check for an authorized domain
        let mut potential_domains = Vec::new();

        for s in all_supervisors {
            let tools = crate::backend::get_supervisor_tools(db, s.id)
                .await
                .unwrap_or_default();
            // If any tool in this domain allows the intent, this domain is a candidate
            if tools.iter().any(|t| {
                let g = crate::engine::orchestrator::guardrails::check_tool_guardrails(
                    &original_intent,
                    "",
                    t,
                    &serde_json::json!({}),
                );
                g.allowed
            }) {
                potential_domains.push(s);
            }
        }

        // Sort by priority or most recent activity (heuristic: if history has it, prefer it)
        if !potential_domains.is_empty() {
            potential_domains.sort_by(|a, b| b.routing_priority.cmp(&a.routing_priority));
            let winner = potential_domains.remove(0);
            return Ok(OrchestrationDecision {
                route: OrchestrationRoute::SingleDomain,
                domain: Some(winner.domain.clone()),
                personality_mode: "operator".to_string(),
                reason: format!(
                    "Domain '{}' authorized by tool guardrails for this intent.",
                    winner.domain
                ),
            });
        }
    }

    let heuristic = heuristic_decision(db, query, has_artifacts).await;
    let is_greeting = ["hi", "hello", "hey", "how are you", "who are you"]
        .iter()
        .any(|&g| query.to_lowercase().starts_with(g));

    if matches!(heuristic.route, OrchestrationRoute::DirectChat)
        && (query.len() < 40 || is_greeting)
    {
        return Ok(heuristic);
    }

    let Some(orchestrator_agent) = all_agents.iter().find(|a| a.domain == "orchestrator") else {
        return Ok(heuristic);
    };

    let orchestrator_model = crate::backend::resolve_model_name(
        db,
        Some(&orchestrator_agent.model),
        "default_chat_model",
        "qwen2.5:7b-instruct",
    )
    .await;

    let decision_agent =
        rig::agent::AgentBuilder::new(ollama_client.completion_model(&orchestrator_model))
            .preamble(&format!(
                "{}\n\nYou are the routing control layer for Overseer.\n\
                 Decide how the request should be handled.\n\
                 Return JSON only with this shape:\n\
                 {{\"route\":\"direct_chat|single_domain|multi_step\",\"domain\":\"chat|web|stealth|desktop|computer|office|vision|neontune|null\",\"personality_mode\":\"assistant|analyst|operator|vision|orchestrator\",\"reason\":\"short explanation\"}}\n\
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

    let prompt = if has_artifacts {
        let artifact_hint = crate::backend::get_setting_or(
            db,
            "artifact_routing_hint",
            "[ARTIFACTS ATTACHED] The user has selected local file artifacts as context. Route to 'chat' domain so the agent can answer from the attached content. Do NOT route to 'stealth' (search) unless the user explicitly asks to search the web.",
        ).await;
        format!(
            "{}\n\nUSER_REQUEST:\n{}\n\nRECENT_CONTEXT:\n{}\n\nReturn only JSON.",
            artifact_hint,
            query,
            if memory_context.is_empty() {
                "No recent context.".to_string()
            } else {
                memory_context.to_string()
            }
        )
    } else {
        format!(
            "USER_REQUEST:\n{}\n\nRECENT_CONTEXT:\n{}\n\nReturn only JSON.",
            query,
            if memory_context.is_empty() {
                "No recent context.".to_string()
            } else {
                memory_context.to_string()
            }
        )
    };

    let raw = match decision_agent.prompt(&prompt).await {
        Ok(raw) => raw,
        Err(err) => {
            println!(
                "⚠️  ORCHESTRATOR: Model routing unavailable, falling back to heuristic routing: {}",
                err
            );
            return Ok(heuristic);
        }
    };
    let cleaned = crate::engine::utils::clean_response(raw);

    serde_json::from_str::<OrchestrationDecision>(&cleaned)
        .map_err(|e| {
            format!(
                "Routing decision failed to parse: {} for input: {}",
                e, cleaned
            )
        })
        .or_else(|_| Ok(heuristic))
}

pub(crate) async fn record_event(
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

fn infer_artifact_kind_and_content_type(value: &str) -> (String, Option<String>) {
    let trimmed = value.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return ("link".to_string(), Some("text/uri-list".to_string()));
    }

    let path = std::path::Path::new(trimmed);
    let looks_like_file = path.is_absolute()
        || path.exists()
        || (path.extension().is_some()
            && (trimmed.contains('\\') || trimmed.contains('/') || trimmed.contains('.')));

    if looks_like_file {
        let content_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();
        let kind = if content_type.starts_with("image/") {
            "image"
        } else {
            "file"
        };
        return (kind.to_string(), Some(content_type));
    }

    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return ("record".to_string(), Some("application/json".to_string()));
    }

    ("note".to_string(), Some("text/plain".to_string()))
}

fn infer_artifact_label(kind: &str, step: &str, value: &str) -> String {
    let trimmed = value.trim();
    let path = std::path::Path::new(trimmed);

    if matches!(kind, "image" | "file") {
        if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
            return if kind == "image" && name.to_ascii_lowercase().contains("desktop") {
                "Desktop Screenshot".to_string()
            } else {
                name.to_string()
            };
        }
    }

    if kind == "link" {
        return "Linked Artifact".to_string();
    }

    let compact_step = step.trim();
    if !compact_step.is_empty() {
        return compact_step
            .chars()
            .take(72)
            .collect::<String>()
            .trim()
            .to_string();
    }

    trimmed
        .chars()
        .take(72)
        .collect::<String>()
        .trim()
        .to_string()
}

async fn persist_step_artifacts(
    db: &crate::db::Database,
    conversation_id: i64,
    execution_run_id: Option<i64>,
    event_sequence: &mut i64,
    domain: &str,
    step: &str,
    summary: &str,
    artifacts: &[String],
) {
    let mut seen = std::collections::HashSet::new();
    let mut recorded = Vec::new();

    for artifact in artifacts {
        let trimmed = artifact.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }

        let (kind, content_type) = infer_artifact_kind_and_content_type(trimmed);
        let label = infer_artifact_label(&kind, step, trimmed);
        let is_file_like = matches!(kind.as_str(), "image" | "file");
        let file_path = is_file_like.then_some(trimmed);
        let content = if is_file_like {
            Some(summary)
        } else {
            Some(trimmed)
        };
        let metadata_json = serde_json::json!({
            "source": "supervisor_result",
            "domain": domain,
            "step_text": step,
            "raw_value": trimmed,
        })
        .to_string();

        match crate::backend::create_artifact(
            db,
            conversation_id,
            execution_run_id,
            Some(domain),
            Some(step),
            &kind,
            &label,
            content,
            file_path,
            content_type.as_deref(),
            Some(&metadata_json),
        )
        .await
        {
            Ok(_) => recorded.push(serde_json::json!({
                "kind": kind,
                "label": label,
                "value": trimmed,
            })),
            Err(err) => println!("Failed to persist artifact '{}': {}", trimmed, err),
        }
    }

    if !recorded.is_empty() {
        let details = serde_json::json!({
            "count": recorded.len(),
            "artifacts": recorded,
        })
        .to_string();
        record_event(
            db,
            execution_run_id,
            event_sequence,
            "artifacts_recorded",
            "completed",
            Some(domain),
            Some(step),
            None,
            None,
            Some(&format!(
                "Recorded {} artifacts for this step.",
                recorded.len()
            )),
            Some(&details),
        )
        .await;
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
    println!(
        "🔍 ORCHESTRATOR: Received query [len={}]: '{}'",
        query.len(),
        query
    );
    let (clean_query, artifact_refs, work_item_refs) = extract_structured_context(query);
    println!(
        "🔍 ORCHESTRATOR: Extracted {} artifacts, {} work items",
        artifact_refs.len(),
        work_item_refs.len()
    );
    let artifact_context = format_artifact_context(&artifact_refs);
    let work_item_context = format_work_item_context(&work_item_refs);
    let structured_context = [artifact_context.clone(), work_item_context.clone()]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let enriched_memory_context = if structured_context.is_empty() {
        memory_context.to_string()
    } else if memory_context.trim().is_empty() {
        structured_context.clone()
    } else {
        format!("{}\n\n{}", memory_context.trim(), structured_context)
    };
    println!(
        "🔍 ORCHESTRATOR: Enriched memory context [len={}]",
        enriched_memory_context.len()
    );

    let run_id = crate::backend::start_execution_run(db, conversation_id, query)
        .await
        .ok();
    let mut event_sequence = 1_i64;

    let mut has_artifacts = !artifact_refs.is_empty();

    // Also check entire conversation history for attachments (from system.rs).
    // We scan all messages (not just recent) so follow-up messages still know artifacts exist.
    if !has_artifacts {
        for msg in history.iter() {
            if let rig::completion::Message::User { content } = msg {
                for item in content.iter() {
                    if let rig::message::UserContent::Text(text) = item {
                        if text.text.contains("[Attached File:")
                            || text.text.contains("[Attached Image:")
                        {
                            has_artifacts = true;
                            break;
                        }
                    } else if let rig::message::UserContent::Image(_) = item {
                        has_artifacts = true;
                        break;
                    }
                }
            }
            if has_artifacts {
                break;
            }
        }
    }

    // Determine the fast-path domain, or fall back to artifact default domain for follow-up messages
    let fast_path_domain = match discovery::discover_fast_path_domain(
        db,
        &clean_query,
        has_artifacts,
    )
    .await
    {
        Some(domain) => Some(domain),
        None if has_artifacts => {
            // No keyword matched, but artifacts are present (likely a follow-up question about attached files).
            // Fall back to a DB-configurable default domain so the agent retains file context.
            let default_domain =
                crate::backend::get_setting_or(db, "artifact_default_domain", "chat").await;
            println!(
                "🎯 ARTIFACT-FALLBACK: No keyword match, but artifacts present. Routing to default domain '{}'",
                default_domain
            );
            Some(default_domain)
        }
        None => None,
    };

    if let Some(domain) = fast_path_domain {
        println!(
            "🎯 FAST-PATH: Query matched keyword for domain '{}'",
            domain
        );
        let mut browser_pid: Option<i32> = None;
        let status_res = dispatcher::dispatch_domain(
            db,
            run_id,
            &mut event_sequence,
            all_agents,
            &domain,
            "Executing fast-path request",
            &clean_query,
            history.clone(),
            "",                           // _memories
            "Fast-path execution start.", // plan_history
            &enriched_memory_context,
            "Fast-path task", // _macro_task_description
            perception_tool,
            ollama_client,
            model_timeout_ms,
            &mut browser_pid,
        )
        .await;

        return match status_res {
            Ok(r) => r.display_text(),
            Err(e) => format!("Error in fast-path dispatcher: {}", e),
        };
    }

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

    if !artifact_refs.is_empty() {
        let details = serde_json::to_string(&artifact_refs).ok();
        record_event(
            db,
            run_id,
            &mut event_sequence,
            "artifact_context_loaded",
            "completed",
            None,
            None,
            None,
            None,
            Some(&format!(
                "Loaded {} selected artifacts into execution context.",
                artifact_refs.len()
            )),
            details.as_deref(),
        )
        .await;
    }

    if !work_item_refs.is_empty() {
        let details = serde_json::to_string(&work_item_refs).ok();
        record_event(
            db,
            run_id,
            &mut event_sequence,
            "work_item_context_loaded",
            "completed",
            None,
            None,
            None,
            None,
            Some("Loaded selected work items into execution context."),
            details.as_deref(),
        )
        .await;
    }

    if let Some((follow_up_response, details)) =
        resolve_screenshot_follow_up(db, conversation_id, &clean_query).await
    {
        record_event(
            db,
            run_id,
            &mut event_sequence,
            "computer_follow_up_resolved",
            "completed",
            Some("computer"),
            None,
            Some("os_tool"),
            None,
            Some("Resolved screenshot follow-up from the latest assistant attachment."),
            Some(&details),
        )
        .await;
        if let Some(run_id) = run_id {
            let _ = crate::backend::finish_execution_run(
                db,
                run_id,
                "completed",
                Some(&follow_up_response),
                None,
            )
            .await;
        }
        return follow_up_response;
    }

    let decision = match decide_orchestration(
        db,
        all_agents,
        &clean_query,
        &history,
        &enriched_memory_context,
        ollama_client,
        has_artifacts,
    )
    .await
    {
        Ok(decision) => decision,
        Err(err) => {
            let heuristic = heuristic_decision(db, &clean_query, has_artifacts).await;
            if let Some(friendly_error) = user_facing_runtime_error(&err) {
                if heuristic
                    .domain
                    .as_deref()
                    .map(|domain| is_native_computer_fast_path_request(&clean_query, domain))
                    .unwrap_or(false)
                {
                    record_event(
                        db,
                        run_id,
                        &mut event_sequence,
                        "model_backend_bypassed",
                        "completed",
                        heuristic.domain.as_deref(),
                        None,
                        None,
                        None,
                        Some(
                            "Model backend unavailable during orchestration decision; using native computer heuristic fallback.",
                        ),
                        Some(&err),
                    )
                    .await;
                    heuristic
                } else {
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
            } else {
                heuristic
            }
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
                model: String::new(),
                domain: "chat".to_string(),
                version: "1.0.0".to_string(),
                code: None,
                source_files: Vec::new(),
            });
        let chat_model = crate::backend::resolve_model_name(
            db,
            Some(&chat_info.model),
            "default_chat_model",
            "qwen2.5:7b-instruct",
        )
        .await;

        let chat_agent = ChatAgent::new(ollama_client, &chat_model, &chat_info.instructions);
        let direct_context = if enriched_memory_context.is_empty() {
            "No recent context.".to_string()
        } else {
            enriched_memory_context.to_string()
        };

        record_event(
            db,
            run_id,
            &mut event_sequence,
            "direct_chat_started",
            "running",
            Some("chat"),
            Some(&clean_query),
            None,
            Some(&chat_model),
            Some("Routing request directly to chat agent."),
            Some(&decision_details),
        )
        .await;

        let direct_response = chat_agent.answer(&clean_query, &direct_context).await;
        if let Some(friendly_error) = user_facing_runtime_error(&direct_response) {
            record_event(
                db,
                run_id,
                &mut event_sequence,
                "direct_chat_failed",
                "error",
                Some("chat"),
                Some(&clean_query),
                None,
                Some(&chat_model),
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
            Some(&clean_query),
            None,
            Some(&chat_model),
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
            "task": clean_query,
        }])
        .to_string();
        record_event(
            db,
            run_id,
            &mut event_sequence,
            "macro_plan_generated",
            "completed",
            decision.domain.as_deref(),
            Some(&clean_query),
            None,
            None,
            Some("Orchestrator routed request directly to a single domain."),
            Some(&synthetic_plan),
        )
        .await;
        planner::sanitize_plan(&synthetic_plan, &clean_query)
    } else {
        println!("🧠 MASTER PLANNER Thinking...");
        let planner_query = if artifact_context.is_empty() {
            clean_query.clone()
        } else {
            format!("{}\n\n{}", clean_query, artifact_context)
        };
        let plan_text: String = match timeout(
            Duration::from_millis(model_timeout_ms),
            master_planner.plan(&planner_query),
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

        planner::sanitize_plan(&plan_text, &clean_query)
    };

    let mut final_report = String::new();
    let mut browser_pid: Option<i32> = None;

    let safe_memory = enriched_memory_context
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

        let micro_steps: Vec<String> = if is_native_computer_fast_path_request(
            &macro_task.task,
            &macro_task.domain,
        ) {
            record_event(
                db,
                run_id,
                &mut event_sequence,
                "micro_plan_native_fast_path",
                "completed",
                Some(&macro_task.domain),
                Some(&macro_task.task),
                Some("os_tool"),
                None,
                Some("Skipping model planning and using a native computer fast path."),
                None,
            )
            .await;

            vec![macro_task.task.clone()]
        } else if let Some(p) = db_planner {
            println!("💡 DYNAMIC PLANNER: Using '{}' configuration.", p.name);
            let model_name = crate::backend::resolve_model_name(
                db,
                p.model.as_deref(),
                "default_chat_model",
                "qwen2.5:7b-instruct",
            )
            .await;
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
                if is_native_computer_fast_path_request(&macro_task.task, &macro_task.domain) {
                    record_event(
                        db,
                        run_id,
                        &mut event_sequence,
                        "model_backend_bypassed",
                        "completed",
                        Some(&macro_task.domain),
                        Some(&macro_task.task),
                        Some("os_tool"),
                        Some(&model_name),
                        Some(
                            "Model backend unavailable during domain planning; continuing with native computer fast path.",
                        ),
                        Some(&sub_plan),
                    )
                    .await;
                    vec![macro_task.task.clone()]
                } else {
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
            } else {
                planner::parse_micro_steps(&sub_plan, &macro_task.task)
            }
        } else if let Some(agent_info) = all_agents.iter().find(|a| {
            a.domain == planner_domain
                || a.domain == macro_task.domain
                    && (a.name.to_lowercase().contains("planner") || a.domain == "orchestrator")
        }) {
            let model_name = crate::backend::resolve_model_name(
                db,
                Some(&agent_info.model),
                "default_chat_model",
                "qwen2.5:7b-instruct",
            )
            .await;
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
                if is_native_computer_fast_path_request(&macro_task.task, &macro_task.domain) {
                    record_event(
                        db,
                        run_id,
                        &mut event_sequence,
                        "model_backend_bypassed",
                        "completed",
                        Some(&macro_task.domain),
                        Some(&macro_task.task),
                        Some("os_tool"),
                        Some(&model_name),
                        Some(
                            "Model backend unavailable during fallback planning; continuing with native computer fast path.",
                        ),
                        Some(&sub_plan),
                    )
                    .await;
                    vec![macro_task.task.clone()]
                } else {
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
            } else {
                planner::parse_micro_steps(&sub_plan, &macro_task.task)
            }
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
                final_report.push_str(&format!("\n\n{}", result));
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
                        final_report.push_str(&format!("\n\n{}", result));
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
                &clean_query,
                history.clone(),
                &memories,
                &plan_history,
                &enriched_memory_context,
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
                    let mut final_result = if supervisor_result
                        .raw_response
                        .starts_with("---CHART_SVG---")
                        || supervisor_result
                            .raw_response
                            .starts_with("---CHART_HTML---")
                        || supervisor_result
                            .raw_response
                            .starts_with("---CHART_ASSET---")
                    {
                        supervisor_result.raw_response.clone()
                    } else {
                        supervisor_result.display_text()
                    };

                    // Code Splitting: Extracted Grounding Logic
                    supervisor_result = grounding::verify_execution_grounding(
                        db,
                        all_agents,
                        ollama_client,
                        &clean_query,
                        step,
                        &macro_task.domain,
                        supervisor_result.clone(),
                        model_timeout_ms,
                    )
                    .await
                    .unwrap_or(supervisor_result);

                    final_result = if supervisor_result
                        .raw_response
                        .starts_with("---CHART_SVG---")
                        || supervisor_result
                            .raw_response
                            .starts_with("---CHART_HTML---")
                        || supervisor_result
                            .raw_response
                            .starts_with("---CHART_ASSET---")
                    {
                        supervisor_result.raw_response.clone()
                    } else {
                        supervisor_result.display_text()
                    };

                    let event_status = match supervisor_result.status.as_str() {
                        "failed" => "error",
                        _ if supervisor_result.verified => "verified",
                        _ => "unverified",
                    };
                    let supervisor_details = serde_json::to_string(&supervisor_result).ok();

                    if !supervisor_result.artifacts.is_empty() {
                        persist_step_artifacts(
                            db,
                            conversation_id,
                            run_id,
                            &mut event_sequence,
                            &macro_task.domain,
                            step,
                            &final_result,
                            &supervisor_result.artifacts,
                        )
                        .await;
                    }

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
                    final_report.push_str(&format!("\n\n{}", final_result));
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

    let macro_task_domains: Vec<String> =
        macro_tasks.iter().map(|task| task.domain.clone()).collect();
    if should_bypass_final_synthesizer(&macro_task_domains) {
        println!("Bypassing final synthesizer for direct computer/desktop run.");
        if let Some(run_id) = run_id {
            let _ = crate::backend::finish_execution_run(
                db,
                run_id,
                "completed",
                Some(&final_report),
                None,
            )
            .await;
        }
        return final_report;
    }

    let chart_markers = ["---CHART_SVG---", "---CHART_HTML---", "---CHART_ASSET---"];
    for marker in &chart_markers {
        if final_report.contains(marker) {
            println!("📊 CHART MARKER found in final_report — bypassing synthesizer.");
            if let Some(run_id) = run_id {
                let _ = crate::backend::finish_execution_run(
                    db,
                    run_id,
                    "completed",
                    Some(&final_report),
                    None,
                )
                .await;
            }
            return final_report;
        }
    }

    let orchestrator_agent = all_agents.iter().find(|a| a.domain == "orchestrator");
    let synthesizer_agent = all_agents.iter().find(|a| a.domain == "synthesizer");

    let synthesis_model = crate::backend::resolve_model_name(
        db,
        synthesizer_agent
            .map(|a| a.model.as_str())
            .or_else(|| orchestrator_agent.map(|a| a.model.as_str())),
        "default_chat_model",
        "qwen2.5:7b-instruct",
    )
    .await;

    let synthesis_instructions = synthesizer_agent
        .map(|a| a.instructions.as_str())
        .unwrap_or(
        "You are a research synthesizer. Produce clean, detailed Markdown from research findings.",
    );

    let synthesis_agent =
        rig::agent::AgentBuilder::new(ollama_client.completion_model(&synthesis_model))
            .preamble(synthesis_instructions)
            .build();

    let style = decision
        .domain
        .as_ref()
        .and_then(|d| {
            // Simple heuristic: fetch style for the first domain in the decision
            // In a real multi-step, we might want a more complex aggregation, but for now this works.
            Some(d.clone())
        })
        .unwrap_or_else(|| "chat".to_string());

    let report_style = sqlx::query_scalar::<_, String>(
        "SELECT report_style FROM supervisors WHERE domain = ? LIMIT 1",
    )
    .bind(&style)
    .fetch_optional(&db.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "detailed".to_string());

    let style_instructions = match report_style.as_str() {
        "concise" => "Provide an extremely brief, one-sentence summary of the findings. Ignore minor details.",
        "narrative" => "Provide a conversational, narrative-driven response. Explain the context and findings as a story.",
        _ => "Provide a clear, detailed response to the user. Include all relevant evidence and research findings clearly.",
    };

    let synthesis_prompt = format!(
        "USER_QUERY: {}\n\nRESEARCH FINDINGS:\n{}\n\n\
         Instructions: {}",
        clean_query, final_report, style_instructions
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
        Some(&synthesis_model),
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
        Some(&synthesis_model),
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

#[cfg(test)]
mod tests {
    use super::{
        classify_screenshot_follow_up, infer_native_computer_domain,
        is_native_computer_fast_path_request, should_bypass_final_synthesizer,
        ScreenshotFollowUpIntent,
    };

    #[test]
    fn computer_and_desktop_runs_skip_final_synthesis() {
        assert!(should_bypass_final_synthesizer(&["computer".to_string()]));
        assert!(should_bypass_final_synthesizer(&[
            "computer".to_string(),
            "desktop".to_string()
        ]));
        assert!(!should_bypass_final_synthesizer(&[
            "computer".to_string(),
            "chat".to_string()
        ]));
        assert!(!should_bypass_final_synthesizer(&[]));
    }

    #[test]
    fn screenshot_follow_up_intents_are_classified() {
        assert_eq!(
            classify_screenshot_follow_up("where is it?"),
            Some(ScreenshotFollowUpIntent::Locate)
        );
        assert_eq!(
            classify_screenshot_follow_up("send it here"),
            Some(ScreenshotFollowUpIntent::Resend)
        );
        assert_eq!(classify_screenshot_follow_up("hello there"), None);
    }

    #[test]
    fn native_computer_domain_handles_loose_local_action_phrasing() {
        assert_eq!(
            infer_native_computer_domain("Take a screeshot on my pc"),
            Some("computer".to_string())
        );
        assert_eq!(
            infer_native_computer_domain("Take a open discord on my pc"),
            Some("computer".to_string())
        );
        assert_eq!(
            infer_native_computer_domain("Inspect accessibility tree for the current window"),
            Some("computer".to_string())
        );
        assert_eq!(
            infer_native_computer_domain("Focus element Save button"),
            Some("computer".to_string())
        );
    }

    #[test]
    fn native_computer_fast_paths_cover_offline_local_tasks() {
        assert!(is_native_computer_fast_path_request(
            "Take a screeshot on my pc",
            "computer"
        ));
        assert!(is_native_computer_fast_path_request(
            "Take a open discord on my pc",
            "computer"
        ));
        assert!(is_native_computer_fast_path_request(
            "What is currently running on my computer",
            "computer"
        ));
        assert!(is_native_computer_fast_path_request(
            "Find my resume path on this PC",
            "computer"
        ));
        assert!(is_native_computer_fast_path_request(
            "Which browser windows are open right now?",
            "computer"
        ));
        assert!(is_native_computer_fast_path_request(
            "Type hello@example.com into the Email field",
            "computer"
        ));
        assert!(!is_native_computer_fast_path_request(
            "Write me a poem about the ocean",
            "chat"
        ));
    }
}
