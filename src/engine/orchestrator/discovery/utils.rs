// src/engine/orchestrator/discovery/utils.rs

pub fn looks_like_planner_request(query: &str) -> bool {
    let lower = query.to_lowercase();
    lower.contains("workflow") || lower.contains("traceability") || lower.starts_with("plan ")
        || lower.contains(" plan for ") || lower.contains("create a plan")
}

pub fn keyword_score(keyword: &str) -> usize {
    let base = keyword.len() + (keyword.split_whitespace().count() * 100);
    if is_technical_token(keyword) { base + 1000 } else { base }
}

fn is_technical_token(token: &str) -> bool {
    matches!(token.to_lowercase().as_str(), 
        "pdf" | "docx" | "xlsx" | "csv" | "chart" | "document" | "report" | "search" | "web_search"
    )
}
