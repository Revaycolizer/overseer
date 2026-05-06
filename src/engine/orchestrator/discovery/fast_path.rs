// src/engine/orchestrator/discovery/fast_path.rs
use crate::db::Database;
use super::utils::{looks_like_planner_request, keyword_score};

pub async fn discover_fast_path_domain(
    db: &Database,
    query: &str,
    has_artifacts: bool,
) -> Option<String> {
    let lower_query = query.to_lowercase();
    if looks_like_planner_request(&lower_query) { return None; }

    let supervisors = match sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors WHERE (fast_path_keywords IS NOT NULL AND fast_path_keywords != '') \
         OR (artifact_fast_path_keywords IS NOT NULL AND artifact_fast_path_keywords != '') \
         ORDER BY routing_priority DESC",
    ).fetch_all(&db.pool).await {
        Ok(s) => s,
        Err(e) => { eprintln!("Error fetching supervisors: {}", e); return None; }
    };

    let mut best_match: Option<(String, usize, i32)> = None;
    for supervisor in &supervisors {
        if has_artifacts && supervisor.skip_fast_path_if_artifacts { continue; }
        
        let keywords = if has_artifacts && supervisor.artifact_fast_path_keywords.is_some() {
            supervisor.artifact_fast_path_keywords.as_ref().unwrap()
        } else if let Some(ref k) = supervisor.fast_path_keywords {
            k
        } else { continue };

        for keyword_raw in keywords.split(',') {
            let keyword = keyword_raw.trim();
            if keyword.is_empty() { continue; }
            let pattern = format!(r"(?i)\b{}\b", regex::escape(keyword));
            if let Ok(re) = regex::Regex::new(&pattern) {
                if re.is_match(&lower_query) {
                    let score = keyword_score(keyword);
                    if should_replace(&best_match, score, supervisor.routing_priority) {
                        best_match = Some((supervisor.domain.clone(), score, supervisor.routing_priority));
                    }
                }
            }
        }
    }

    if let Some((domain, _, _)) = best_match {
        println!("FAST-PATH: Query matched best keyword cluster for domain '{}'", domain);
        return Some(domain);
    }
    None
}

fn should_replace(current: &Option<(String, usize, i32)>, score: usize, priority: i32) -> bool {
    current.as_ref().map(|(_, best_score, best_priority)| {
        score > *best_score || (score == *best_score && priority > *best_priority)
    }).unwrap_or(true)
}
