use regex as re;
use serde_json;
use rig::tool::Tool;
use crate::engine::tools::{
    StructuredPerceptionTool, PerceptionArgs,
    PyUITool, PyUIArgs,
    CodeExecutorTool, CodeExecutorArgs,
    BrowserTool, BrowserArgs,
    StealthBrowseTool, StealthBrowseArgs,
};

pub async fn execute_tools(
    db: &crate::db::Database,
    response_str: &str, 
    step: &str,
    perception_tool: &StructuredPerceptionTool,
    last_action_x: &mut Option<i32>,
    last_action_y: &mut Option<i32>,
    last_action_text: &mut Option<String>,
    last_action_type: &mut String,
    last_action_had_visual_change: &mut bool,
    browser_pid: &mut Option<i32>,
    last_action_code: &mut Option<String>,
    is_web_domain: bool,
) -> Option<String> {
    let mut outputs = Vec::new();
    let mut json_candidates = Vec::new();
    let mut brace_stack = 0;
    let mut start = 0;
    
    for (byte_idx, c) in response_str.char_indices() {
        match c {
            '{' => {
                if brace_stack == 0 { start = byte_idx; }
                brace_stack += 1;
            },
            '}' => {
                if brace_stack > 0 {
                    brace_stack -= 1;
                    if brace_stack == 0 {
                        let end = byte_idx + c.len_utf8();
                        json_candidates.push(&response_str[start..end]);
                    }
                }
            },
            _ => {}
        }
    }

    for candidate in json_candidates {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(candidate) {
            println!("DEBUG: Checking JSON candidate: {}", candidate);
            if let Some(tool_calls) = val.get("tool_calls").and_then(|v| v.as_array()) {
                for tc in tool_calls {
                    process_json_value(db, tc, is_web_domain, &mut outputs, step, perception_tool, last_action_x, last_action_y, last_action_text, last_action_type, last_action_had_visual_change, browser_pid, last_action_code).await;
                }
            } else {
                process_json_value(db, &val, is_web_domain, &mut outputs, step, perception_tool, last_action_x, last_action_y, last_action_text, last_action_type, last_action_had_visual_change, browser_pid, last_action_code).await;
            }
        }
    }

    // Fallback: Legacy extraction
    let re_perc = re::Regex::new(r"`structured_perception` action='([^']+)'(?:\s+app='([^']+)')?").unwrap();
    for cap in re_perc.captures_iter(response_str) {
        let action = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let app = cap.get(2).map(|m| m.as_str().to_string());
        match perception_tool.call(PerceptionArgs { action, app }).await {
            Ok(out) => outputs.push(format!("Tool structured_perception output: {}", out)),
            Err(e) => outputs.push(format!("Tool structured_perception error: {}", e)),
        }
    }

    if outputs.is_empty() { None } else { Some(outputs.join("\n\n")) }
}

pub async fn process_json_value(
    db: &crate::db::Database,
    val: &serde_json::Value,
    is_web_domain: bool,
    outputs: &mut Vec<String>,
    _step: &str,
    _perception_tool: &StructuredPerceptionTool,
    last_action_x: &mut Option<i32>,
    last_action_y: &mut Option<i32>,
    last_action_text: &mut Option<String>,
    last_action_type: &mut String,
    last_action_had_visual_change: &mut bool,
    browser_pid: &mut Option<i32>,
    last_action_code: &mut Option<String>,
) {
    let params = val.get("parameters")
        .or_else(|| val.get("arguments"))
        .or_else(|| val.get("args"))
        .or_else(|| val.get("params"))
        .or_else(|| if val.is_object() { Some(val) } else { None });
    
    if let Some(params) = params {
        if let Some(obs) = val.get("observation").or_else(|| params.get("observation")).and_then(|v| v.as_str()) {
            println!("👁️ OBSERVATION: {}", obs);
        }
        if let Some(intent) = val.get("intent").or_else(|| params.get("intent")).and_then(|v| v.as_str()) {
            println!("🎯 INTENT: {}", intent);
        }
        let name = val.get("name")
            .or_else(|| val.get("tool_call"))
            .or_else(|| val.get("tool"))
            .or_else(|| val.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        
        let mut mapped_name = match name {
            "click" | "type" | "search" | "launch" | "focus" | "scroll" | "play_song" 
                => "ui_automation",
            "get_ui_tree" | "get_delta" | "wait_for_settle" | "get_screen_text"
                => "structured_perception",
            "code_executor" | "execute_code" | "run_code" | "calculate" | "math"
                => "code_executor",
            "http_request" | "http" | "fetch" | "api_call" | "web_request"
                => "http_request",
            "stealth_search" | "search_stealth" | "background_search"
                => "stealth_search",
            "stealth_browse" | "browse_stealth" | "background_browse" | "get_page_text"
                => "stealth_browse",
            "done" | "finish" | "complete" => "DONE",
            "browser_automation" | "browser" => "browser_automation",
            _ => "unknown",
        };

        let default_action = if mapped_name == "ui_automation" || mapped_name == "structured_perception" {
            Some(name)
        } else {
            None
        };

        if is_web_domain {
            if mapped_name == "ui_automation" {
                mapped_name = "browser_automation";
            } else if mapped_name == "unknown" && params.get("action").is_some() {
                mapped_name = "browser_automation";
            }
        } else {
            if mapped_name == "unknown" && (params.get("action").is_some() || params.get("app").is_some()) {
                mapped_name = "ui_automation";
            }
        }

        let tools_list = crate::backend::get_tools(db).await.unwrap_or_default();
        let get_script = |tname: &str| -> Option<String> {
            if let Some(tool) = tools_list.iter().find(|t| t.name == tname) {
                crate::engine::utils::discovery::get_tool_executable_path(tool)
            } else {
                None
            }
        };

        let real_name = mapped_name;
        if real_name == "DONE" {
            outputs.push("TASK_COMPLETED".to_string());
            return;
        }

        // ── CODE EXECUTOR ──
        if real_name == "code_executor" {
            let tool = CodeExecutorTool::new();
            let mut language = params.get("language").and_then(|v| v.as_str()).unwrap_or("python").to_string();
            let code = params.get("code").or_else(|| params.get("command")).and_then(|v| v.as_str()).unwrap_or("").to_string();
            
            // Auto-detect bash for common shell commands
            if language == "python" && (code.starts_with("libreoffice") || code.starts_with("rm ") || code.starts_with("mkdir ") || code.starts_with("ls ") || code.starts_with("wget ") || code.starts_with("git ")) {
                language = "bash".to_string();
                println!("🔀 AUTO-DETECT: Switched code_executor to 'bash' based on command: {}", code);
            }
            
            if let Some(last_code) = last_action_code.as_ref() {
                if last_code == &code && *last_action_had_visual_change {
                    outputs.push("REDUNDANT CODE EXECUTION: Already succeeded.".to_string());
                    return;
                }
            }
            let args = CodeExecutorArgs { language, code: code.clone() };
            match tool.call(args).await {
                Ok(out) => {
                    outputs.push(format!("Tool code_executor output: {}", out));
                    *last_action_code = Some(code);
                    let out_lower = out.to_lowercase();
                    *last_action_had_visual_change = !(out_lower.contains("error:") || out_lower.contains("failed:"));
                },
                Err(e) => { outputs.push(format!("Tool code_executor error: {}", e)); *last_action_had_visual_change = false; },
            }
            return;
        }

        // ── BROWSER AUTOMATION ──
        if real_name == "browser_automation" {
            let action = params.get("action").and_then(|v| v.as_str()).or(default_action).unwrap_or("get_title").to_string();
            let url    = params.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
            let text   = params.get("text").or_else(|| params.get("target")).or_else(|| params.get("query")).and_then(|v| v.as_str()).map(|s| s.to_string());
            
            if action == "click" || action == "scroll" || action == "search" || action == "type" || action == "navigate" {
                let same_text = if action == "navigate" { url.is_some() && url.as_deref() == last_action_text.as_deref() } else { text.is_some() && last_action_text.as_ref() == text.as_ref() };
                if *last_action_type == action && same_text && *last_action_had_visual_change {
                    outputs.push(format!("REDUNDANT BROWSER ACTION: Already performed '{}'.", action));
                    return;
                }
            }

            let args = BrowserArgs { 
                action: action.clone(), url: url.clone(), text: text.clone(), 
                selector: params.get("selector").or_else(|| params.get("css")).and_then(|v| v.as_str()).map(|s| s.to_string()), 
                submit: params.get("submit").and_then(|v| v.as_bool()), 
                direction: params.get("direction").and_then(|v| v.as_str()).map(|s| s.to_string()), 
                amount: params.get("amount").and_then(|v| v.as_i64()).map(|v| v as i32), 
                navigate: params.get("navigate").and_then(|v| v.as_bool()), 
                x: params.get("x").and_then(|v| v.as_f64()), y: params.get("y").and_then(|v| v.as_f64()) 
            };
            let tool = BrowserTool::new(get_script("browser_automation"));
            match tool.call(args).await {
                Ok(out) => {
                    let mut json_str = "";
                    for line in out.lines() { if line.trim().starts_with('{') { json_str = line.trim(); break; } }
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(pid) = json_val.get("browser_pid").and_then(|v| v.as_i64()) { *browser_pid = Some(pid as i32); }
                        let status = json_val.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        *last_action_had_visual_change = status == "clicked" || status == "scrolled" || status == "navigated" || status == "typed" || status == "search_submitted" || status == "search_via_ddg" || out.contains("✅");
                    } else {
                        *last_action_had_visual_change = out.contains("✅");
                    }
                    *last_action_type = action.clone();
                    *last_action_text = if action == "navigate" { url } else { text };
                    outputs.push(format!("Tool browser_automation output: {}", out));
                },
                Err(e) => { outputs.push(format!("Tool browser_automation error: {}", e)); *last_action_had_visual_change = false; },
            }
            return;
        }

        // ── UI AUTOMATION (Desktop) ──
        if real_name == "ui_automation" {
            let action = params.get("action").and_then(|v| v.as_str()).or(default_action).unwrap_or("launch").to_string();
            let app    = params.get("app").and_then(|v| v.as_str()).map(|s| s.to_string());
            let text   = params.get("text").or_else(|| params.get("target")).and_then(|v| v.as_str()).map(|s| s.to_string());
            let x      = params.get("x").and_then(|v| v.as_i64()).map(|v| v as i32);
            let y      = params.get("y").and_then(|v| v.as_i64()).map(|v| v as i32);
            let _snap_app = app.clone().unwrap_or_else(|| "active".to_string());

            if action == "click" && *last_action_type == "click" && *last_action_had_visual_change {
                if (text.is_some() && last_action_text.as_ref() == text.as_ref() && *last_action_x == x && *last_action_y == y) || 
                   (text.is_none() && *last_action_x == x && *last_action_y == y) {
                    outputs.push(format!("REDUNDANT CLICK: Already clicked at ({:?}, {:?}).", x, y));
                    return;
                }
            }

            let args = PyUIArgs { action: action.clone(), app: app.clone(), text: text.clone(), x, y, key: params.get("key").and_then(|v| v.as_str()).map(|s| s.to_string()), no_launch: None, clear: params.get("clear").and_then(|v| v.as_bool()), submit: params.get("submit").and_then(|v| v.as_bool()) };
            let tool = PyUITool::new(get_script("ui_automation"));
            match tool.call(args).await {
                Ok(out) => {
                    outputs.push(format!("Tool ui_automation output: {}", out));
                    *last_action_had_visual_change = action == "click" || action == "type" || action == "launch";
                    *last_action_type = action;
                    *last_action_text = text;
                    *last_action_x = x;
                    *last_action_y = y;
                },
                Err(e) => { outputs.push(format!("Tool ui_automation error: {}", e)); *last_action_had_visual_change = false; },
            }
            return;
        }

        // ── STRUCTURED PERCEPTION ──
        if real_name == "structured_perception" {
            let action = params.get("action").and_then(|v| v.as_str()).or(default_action).unwrap_or("get_ui_tree").to_string();
            let app    = params.get("app").and_then(|v| v.as_str()).map(|s| s.to_string());
            let args = PerceptionArgs { action: action.clone(), app };
            let tool = StructuredPerceptionTool::new(db.clone());
            match tool.call(args).await {
                Ok(out) => outputs.push(format!("Tool structured_perception output: {}", out)),
                Err(e) => outputs.push(format!("Tool structured_perception error: {}", e)),
            }
            return;
        }

        // ── HTTP REQUEST ──
        if real_name == "http_request" {
            let tool = crate::engine::tools::HttpTool::new();
            
            // Handle case where params are nested in "query" or "arguments"
            let inner_params = params.get("query").filter(|v| v.is_object())
                .or_else(|| params.get("arguments").filter(|v| v.is_object()))
                .unwrap_or(params);

            let method = inner_params.get("method").and_then(|v| v.as_str())
                .or_else(|| params.get("method").and_then(|v| v.as_str()))
                .unwrap_or("GET").to_string();
                
            let url = inner_params.get("url").and_then(|v| v.as_str())
                .or_else(|| params.get("url").and_then(|v| v.as_str()))
                .or_else(|| params.get("query").and_then(|v| v.as_str()))
                .unwrap_or("").to_string();
                
            let body = inner_params.get("body").and_then(|v| v.as_str())
                .or_else(|| params.get("body").and_then(|v| v.as_str()))
                .map(|s| s.to_string());
                
            let args = crate::engine::tools::HttpArgs { method, url, body, headers: None };
            match tool.call(args).await {
                Ok(out) => outputs.push(format!("Tool http_request output: {}", out)),
                Err(e) => outputs.push(format!("Tool http_request error: {}", e)),
            }
            return;
        }

        // ── STEALTH SEARCH ──
        if real_name == "stealth_search" {
            let tool = crate::engine::tools::StealthSearchTool::new(get_script("stealth_search"));
            let query = params.get("query").and_then(|v| if v.is_object() { v.get("query") } else { Some(v) })
                .or_else(|| params.get("text"))
                .and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = crate::engine::tools::StealthSearchArgs { query };
            match tool.call(args).await {
                Ok(out) => outputs.push(format!("Tool stealth_search output: {}", out)),
                Err(e) => outputs.push(format!("Tool stealth_search error: {}", e)),
            }
            return;
        }

        // ── STEALTH BROWSE ──
        if real_name == "stealth_browse" {
            let tool = StealthBrowseTool::new(get_script("stealth_browse"));
            let url = params.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = StealthBrowseArgs { url };
            match tool.call(args).await {
                Ok(out) => outputs.push(format!("Tool stealth_browse output: {}", out)),
                Err(e) => outputs.push(format!("Tool stealth_browse error: {}", e)),
            }
            return;
        }
    }
}
