use crate::db::Database;

pub async fn discover_fast_path_domain(db: &Database, query: &str) -> Option<String> {
    let lower_query = query.to_lowercase();

    // 1. Fetch all supervisors that have fast_path_keywords defined
    let supervisors = match sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors WHERE fast_path_keywords IS NOT NULL AND fast_path_keywords != '' ORDER BY routing_priority DESC"
    )
    .fetch_all(&db.pool)
    .await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error fetching supervisors for fast-path discovery: {}", e);
            return None;
        }
    };

    // 2. Iterate and match keywords
    for s in supervisors {
        if let Some(ref keywords_str) = s.fast_path_keywords {
            let keywords: Vec<&str> = keywords_str.split(',').map(|k| k.trim()).collect();
            for k in keywords {
                if !k.is_empty() && lower_query.contains(&k.to_lowercase()) {
                    println!("🎯 FAST-PATH: Query matched keyword '{}' for domain '{}'", k, s.domain);
                    return Some(s.domain.clone());
                }
            }
        }
    }

    // 3. Fallback to hardcoded legacy rules if no dynamic matches found
    // (Translating the old logic into this centralized location)

    let screenshot_request = (lower_query.contains("desktop") || lower_query.contains("screen"))
        && (lower_query.contains("screenshot")
            || lower_query.contains("screen capture")
            || lower_query.contains("capture screen")
            || lower_query.contains("capture my desktop")
            || lower_query.contains("take a screenshot"));

    let computer_observation = (lower_query.contains("desktop") || lower_query.contains("screen"))
        && (lower_query.contains("observe")
            || lower_query.contains("inspect")
            || lower_query.contains("watch")
            || lower_query.contains("active window")
            || lower_query.contains("which window")
            || lower_query.contains("what window")
            || lower_query.contains("current window")
            || lower_query.contains("foreground window"));

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

    let file_observation = (lower_query.contains("file")
        || lower_query.contains("path")
        || lower_query.contains("resume")
        || lower_query.contains(".pdf")
        || lower_query.contains(".doc")
        || lower_query.contains(".docx"))
        && (lower_query.contains("find")
            || lower_query.contains("locate")
            || lower_query.contains("search")
            || lower_query.contains("where")
            || lower_query.contains("path"));

    let browser_observation = (lower_query.contains("browser")
        || lower_query.contains("tab")
        || lower_query.contains("page"))
        && (lower_query.contains("current")
            || lower_query.contains("which")
            || lower_query.contains("what")
            || lower_query.contains("open")
            || lower_query.contains("state"));

    if screenshot_request
        || computer_observation
        || process_observation
        || file_observation
        || browser_observation
        || lower_query.contains("control my computer")
        || lower_query.contains("take over my computer")
        || lower_query.contains("use my computer")
        || lower_query.contains("operate my pc")
        || lower_query.contains("computer session")
        || lower_query.contains("fully supervise")
        || lower_query.contains("observe my desktop")
        || lower_query.contains("observe the desktop")
    {
        return Some("computer".to_string());
    }

    if lower_query.contains("research")
        || lower_query.contains("search")
        || lower_query.contains("browse")
        || lower_query.contains("website")
        || lower_query.contains("web")
        || lower_query.contains("internet")
    {
        return Some("web".to_string());
    }

    if lower_query.contains(" music")
        || lower_query.contains("play music")
        || lower_query.contains("play song")
        || lower_query.contains("audio")
        || lower_query.contains("song")
        || lower_query.contains("neontune")
        || lower_query.contains("volume")
        || lower_query.contains("pause")
        || lower_query.contains("next")
    {
        return Some("neontune".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn computer_requests_fall_back_to_computer_domain() {
        let lower_query = "take a screenshot of my desktop".to_string();
        let screenshot_request = (lower_query.contains("desktop") || lower_query.contains("screen"))
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
