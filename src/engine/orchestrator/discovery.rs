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
    
    let computer_observation = (lower_query.contains("desktop") || lower_query.contains("screen"))
        && (lower_query.contains("observe")
            || lower_query.contains("inspect")
            || lower_query.contains("watch")
            || lower_query.contains("active window")
            || lower_query.contains("which window")
            || lower_query.contains("what window")
            || lower_query.contains("current window")
            || lower_query.contains("foreground window"));

    if computer_observation
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
        || lower_query.contains("play")
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
