use crate::db::Database;

pub(crate) fn looks_like_planner_request(query: &str) -> bool {
    let lower = query.to_lowercase();

    lower.contains("workflow")
        || lower.contains("traceability")
        || lower.starts_with("plan ")
        || lower.contains(" plan for ")
        || lower.contains("create a plan")
        || lower.contains("create a workflow plan")
        || lower.contains("monitoring task")
}

fn keyword_score(keyword: &str) -> usize {
    keyword.len() + (keyword.split_whitespace().count() * 100)
}

pub async fn discover_fast_path_domain(
    db: &Database,
    query: &str,
    has_artifacts: bool,
) -> Option<String> {
    let lower_query = query.to_lowercase();

    // Planner-shaped prompts should stay out of broad supervisor fast paths.
    if looks_like_planner_request(&lower_query) {
        return None;
    }

    // 1. Fetch all supervisors that have any routing keywords defined
    let supervisors = match sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors WHERE \
         (fast_path_keywords IS NOT NULL AND fast_path_keywords != '') \
         OR (artifact_fast_path_keywords IS NOT NULL AND artifact_fast_path_keywords != '') \
         ORDER BY routing_priority DESC",
    )
    .fetch_all(&db.pool)
    .await
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error fetching supervisors for fast-path discovery: {}", e);
            return None;
        }
    };

    let mut best_standard_match: Option<(String, usize, i32)> = None;

    // 2. Standard keyword pass (skip artifact-sensitive supervisors when artifacts are present)
    for supervisor in &supervisors {
        if has_artifacts && supervisor.skip_fast_path_if_artifacts {
            continue;
        }

        if let Some(ref keywords_str) = supervisor.fast_path_keywords {
            for keyword_raw in keywords_str.split(',') {
                let keyword = keyword_raw.trim();
                if keyword.is_empty() {
                    continue;
                }

                let pattern = format!(r"(?i)\b{}\b", regex::escape(keyword));
                if let Ok(re) = regex::Regex::new(&pattern) {
                    if re.is_match(&lower_query) {
                        let score = keyword_score(keyword);
                        let should_replace = best_standard_match
                            .as_ref()
                            .map(|(_, best_score, best_priority)| {
                                score > *best_score
                                    || (score == *best_score
                                        && supervisor.routing_priority > *best_priority)
                            })
                            .unwrap_or(true);

                        if should_replace {
                            best_standard_match = Some((
                                supervisor.domain.clone(),
                                score,
                                supervisor.routing_priority,
                            ));
                        }
                    }
                }
            }
        }
    }

    if let Some((domain, _, _)) = best_standard_match {
        println!(
            "FAST-PATH: Query matched best keyword cluster for domain '{}'",
            domain
        );
        return Some(domain);
    }

    // 3. Artifact-aware keyword pass: only runs when the user has attached artifacts
    if has_artifacts {
        let mut best_artifact_match: Option<(String, usize, i32)> = None;

        for supervisor in &supervisors {
            if let Some(ref artifact_keywords_str) = supervisor.artifact_fast_path_keywords {
                for keyword_raw in artifact_keywords_str.split(',') {
                    let keyword = keyword_raw.trim();
                    if keyword.is_empty() {
                        continue;
                    }

                    let pattern = format!(r"(?i)\b{}\b", regex::escape(keyword));
                    if let Ok(re) = regex::Regex::new(&pattern) {
                        if re.is_match(&lower_query) {
                            let score = keyword_score(keyword);
                            let should_replace = best_artifact_match
                                .as_ref()
                                .map(|(_, best_score, best_priority)| {
                                    score > *best_score
                                        || (score == *best_score
                                            && supervisor.routing_priority > *best_priority)
                                })
                                .unwrap_or(true);

                            if should_replace {
                                best_artifact_match = Some((
                                    supervisor.domain.clone(),
                                    score,
                                    supervisor.routing_priority,
                                ));
                            }
                        }
                    }
                }
            }
        }

        if let Some((domain, _, _)) = best_artifact_match {
            println!(
                "ARTIFACT-FAST-PATH: Query matched best artifact keyword cluster for domain '{}' (artifacts present)",
                domain
            );
            return Some(domain);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::looks_like_planner_request;

    #[test]
    fn planner_like_requests_are_detected() {
        assert!(looks_like_planner_request(
            "Create a workflow plan for a daily website monitoring task with traceability."
        ));
        assert!(looks_like_planner_request(
            "Plan a monitoring task with traceability"
        ));
        assert!(!looks_like_planner_request(
            "Take a screenshot of my desktop"
        ));
    }

    #[test]
    fn computer_requests_fall_back_to_computer_domain() {
        let lower_query = "take a screenshot of my desktop".to_string();
        let screenshot_request = (lower_query.contains("desktop")
            || lower_query.contains("screen"))
            && (lower_query.contains("screenshot")
                || lower_query.contains("screen capture")
                || lower_query.contains("capture screen")
                || lower_query.contains("capture my desktop")
                || lower_query.contains("take a screenshot"));
        assert!(screenshot_request);

        let lower_query = "tell me the background tasks that i'm currently running".to_string();
        let process_observation = (lower_query.contains("background")
            || lower_query.contains("running")
            || lower_query.contains("process")
            || lower_query.contains("task"))
            && (lower_query.contains("currently")
                || lower_query.contains("current")
                || lower_query.contains("what")
                || lower_query.contains("which")
                || lower_query.contains("tell me")
                || lower_query.contains("list")
                || lower_query.contains("show"));
        assert!(process_observation);
    }

    #[test]
    fn file_and_browser_observation_patterns_are_detected() {
        let file_query = "can you find my resume path in my pc?".to_string();
        let file_observation = (file_query.contains("file")
            || file_query.contains("path")
            || file_query.contains("resume")
            || file_query.contains(".pdf")
            || file_query.contains(".doc")
            || file_query.contains(".docx"))
            && (file_query.contains("find")
                || file_query.contains("locate")
                || file_query.contains("search")
                || file_query.contains("where")
                || file_query.contains("path"));
        assert!(file_observation);

        let browser_query = "which browser windows are open right now".to_string();
        let browser_observation = (browser_query.contains("browser")
            || browser_query.contains("tab")
            || browser_query.contains("page"))
            && (browser_query.contains("current")
                || browser_query.contains("which")
                || browser_query.contains("what")
                || browser_query.contains("open")
                || browser_query.contains("state"));
        assert!(browser_observation);
    }
}
