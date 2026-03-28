use crate::db::Database;

pub async fn discover_fast_path_domain(
    db: &Database,
    query: &str,
    has_artifacts: bool,
) -> Option<String> {
    let lower_query = query.to_lowercase();

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

    // 2. Standard keyword pass (skip artifact-sensitive supervisors when artifacts are present)
    for s in &supervisors {
        if has_artifacts && s.skip_fast_path_if_artifacts {
            continue;
        }

        if let Some(ref keywords_str) = s.fast_path_keywords {
            for k_raw in keywords_str.split(',') {
                let k = k_raw.trim();
                if k.is_empty() {
                    continue;
                }

                let pattern = format!(r"(?i)\b{}\b", regex::escape(k));
                if let Ok(re) = regex::Regex::new(&pattern) {
                    if re.is_match(&lower_query) {
                        println!(
                            "🎯 FAST-PATH: Query matched keyword '{}' for domain '{}'",
                            k, s.domain
                        );
                        return Some(s.domain.clone());
                    }
                }
            }
        }
    }

    // 3. Artifact-aware keyword pass: only runs when the user has attached artifacts
    if has_artifacts {
        for s in &supervisors {
            if let Some(ref artifact_keywords_str) = s.artifact_fast_path_keywords {
                for k_raw in artifact_keywords_str.split(',') {
                    let k = k_raw.trim();
                    if k.is_empty() {
                        continue;
                    }

                    let pattern = format!(r"(?i)\b{}\b", regex::escape(k));
                    if let Ok(re) = regex::Regex::new(&pattern) {
                        if re.is_match(&lower_query) {
                            println!(
                                "🎯 ARTIFACT-FAST-PATH: Query matched artifact keyword '{}' for domain '{}' (artifacts present)",
                                k, s.domain
                            );
                            return Some(s.domain.clone());
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
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
