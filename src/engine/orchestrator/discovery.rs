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

    None
}
