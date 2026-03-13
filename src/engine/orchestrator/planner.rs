use serde::{Deserialize, Serialize};
use serde_json;
use regex;

#[derive(Deserialize, Serialize, Clone)]
pub struct MacroTask {
    pub domain: String,
    pub task: String,
}

pub fn sanitize_plan(plan_text: &str, query: &str) -> Vec<MacroTask> {
    // Extract JSON array
    let clean_text = plan_text.replace("\\\"", "\"").replace('\n', " ");
    let start = clean_text.find('[');
    let end = clean_text.rfind(']');
    let json_candidate = if let (Some(s), Some(e)) = (start, end) {
        if s < e { clean_text[s..e+1].to_string() } else { clean_text.clone() }
    } else {
        clean_text.clone()
    };

    let mut macro_tasks: Vec<MacroTask> = match serde_json::from_str(&json_candidate) {
        Ok(tasks) => tasks,
        Err(e) => {
            let domain = if query.to_lowercase().contains("neontune") { 
                "neontune" 
            } else if query.to_lowercase().contains("stealth") || query.to_lowercase().contains("research") {
                "stealth"
            } else { 
                "desktop" 
            };
            println!("⚠️ Failed to parse master planner JSON: {}. Routing raw query to {} domain.", e, domain);
            vec![MacroTask { domain: domain.to_string(), task: query.to_string() }]
        }
    };

    // Post-processing: ensure neontune mention goes to neontune domain
    for task in &mut macro_tasks {
        let task_lower = task.task.to_lowercase();
        let query_lower = query.to_lowercase();

        if task_lower.contains("neontune") && task.domain != "neontune" {
            println!("🔄 RE-ROUTING: Detected 'neontune' in task description. Forcing domain to neontune.");
            task.domain = "neontune".to_string();
        }

        // Stealth Safety: If user explicitly said "stealth mode" and planner chose "web", force "stealth".
        if (query_lower.contains("stealth mode") || query_lower.contains("background")) && task.domain == "web" {
            println!("🔄 RE-ROUTING: User requested stealth/background mode but planner chose 'web'. Forcing to 'stealth'.");
            task.domain = "stealth".to_string();
        }

        // Implicit Safety: If domain is "web" but task/query lacks interactive keywords, default to "stealth".
        let interactive_keywords = ["open ", "launch ", "click", "show me", "chrome", "browser", "youtube", "github.com"];
        if task.domain == "web" && !interactive_keywords.iter().any(|k| query_lower.contains(k) || task_lower.contains(k)) {
            println!("🛡️ STEALTH SAFETY: Task marked as 'web' but lacks interactive keywords. Re-routing to 'stealth' for safety.");
            task.domain = "stealth".to_string();
        }
    }

    macro_tasks
}

pub fn parse_micro_steps(plan_text: &str, fallback_query: &str) -> Vec<String> {
    let clean_text = plan_text.replace("\\\"", "\"").replace('\n', " ");
    let s = clean_text.find('[');
    let e = clean_text.rfind(']');
    let json_candidate = if let (Some(s), Some(e)) = (s, e) {
        if s < e { clean_text[s..e+1].to_string() } else { clean_text.clone() }
    } else { clean_text.clone() };

    match serde_json::from_str::<serde_json::Value>(&json_candidate) {
        Ok(serde_json::Value::Array(arr)) => {
            let parsed: Vec<String> = arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            if parsed.is_empty() { vec![fallback_query.to_string()] } else { parsed }
        },
        _ => {
            let s_re = regex::Regex::new(r#""([^"\\]*(?:\\.[^"\\]*)*)""#).unwrap();
            let scavenged: Vec<String> = s_re.captures_iter(&json_candidate)
                .map(|cap| cap[1].replace("\\\"", "\""))
                .filter(|s| s.len() > 3 && !s.contains('{') && !s.contains('}'))
                .collect();
            if scavenged.is_empty() { vec![fallback_query.to_string()] } else { scavenged }
        }
    }
}
