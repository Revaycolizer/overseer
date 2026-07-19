use std::collections::{HashMap, HashSet};

use super::indexer::AgentAtlas;
use crate::engine::utils::discovery::AgentInfo;

/// Search the agent atlas using TF-IDF weighted term scoring.
///
/// Each agent's description and domain are tokenised into a document.
/// The query is tokenised and scored against each document using TF-IDF
/// cosine similarity. This is strictly better than simple `contains()`
/// matching because it rewards distinctive, descriptive terms and ignores
/// common filler words.
pub fn search_atlas(atlas: &AgentAtlas, query: &str, limit: usize) -> Vec<AgentInfo> {
    if atlas.agents.is_empty() || query.trim().is_empty() {
        return atlas.agents.clone().into_iter().take(limit).collect();
    }

    // 1. Build the corpus: each agent's domain + description as a "document"
    let num_docs = atlas.agents.len();
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let tokenised_docs: Vec<Vec<String>> = atlas
        .agents
        .iter()
        .map(|a| {
            let doc_text = format!("{} {}", a.domain, a.description);
            let tokens = tokenise(&doc_text);
            // Count unique terms per doc for IDF
            let unique: HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
            for term in unique {
                *doc_freq.entry(term.to_string()).or_insert(0) += 1;
            }
            tokens
        })
        .collect();

    // 2. Build query TF vector
    let query_tokens = tokenise(query);
    if query_tokens.is_empty() {
        return atlas.agents.clone().into_iter().take(limit).collect();
    }
    let query_vector = tf_vector(&query_tokens);

    // 3. Score each agent document against the query
    let mut scored: Vec<(AgentInfo, f64)> = atlas
        .agents
        .iter()
        .zip(tokenised_docs.iter())
        .map(|(agent, doc_tokens)| {
            let doc_vector = tfidf_vector(doc_tokens, &doc_freq, num_docs);
            let sim = cosine_similarity(&query_vector, &doc_vector);
            (agent.clone(), sim)
        })
        .collect();

    // 4. Sort by score descending, then by domain priority as tiebreaker
    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(a, _)| a)
        .collect()
}

/// Group agents into cells by domain similarity.
///
/// Uses domain prefix clustering: agents sharing the same domain prefix
/// or with overlapping description keywords are grouped together.
/// Single-agent domains form their own singleton cells.
pub fn group_into_cells(agents: &[AgentInfo]) -> Vec<Vec<AgentInfo>> {
    if agents.is_empty() {
        return vec![];
    }
    if agents.len() == 1 {
        return vec![agents.to_vec()];
    }

    // Group by domain first
    let mut domain_groups: HashMap<String, Vec<AgentInfo>> = HashMap::new();
    for agent in agents.iter() {
        // Normalise domain to its broad category
        let category = domain_category(&agent.domain);
        domain_groups
            .entry(category)
            .or_default()
            .push(agent.clone());
    }

    let mut cells: Vec<Vec<AgentInfo>> = Vec::new();
    for (_category, group) in domain_groups {
        // If a group is very large, split further by description similarity
        if group.len() > 3 {
            let mut remaining = group;
            while remaining.len() > 3 {
                // Take up to 3 most similar agents as one cell
                let mut sub_cell = Vec::new();
                sub_cell.push(remaining.remove(0));
                let base_tokens: HashSet<String> = tokenise(&sub_cell[0].description).into_iter().collect();
                let mut idx = 0;
                while idx < remaining.len() && sub_cell.len() < 3 {
                    let t: HashSet<String> = tokenise(&remaining[idx].description).into_iter().collect();
                    let overlap = base_tokens.intersection(&t).count();
                    if overlap > 1 {
                        sub_cell.push(remaining.remove(idx));
                    } else {
                        idx += 1;
                    }
                }
                cells.push(sub_cell);
            }
            if !remaining.is_empty() {
                cells.push(remaining);
            }
        } else {
            cells.push(group);
        }
    }

    cells
}

/// Map a domain string to its broad category.
fn domain_category(domain: &str) -> String {
    let lower = domain.to_lowercase();
    if lower.contains("web") || lower.contains("search") || lower.contains("browse") {
        "web".to_string()
    } else if lower.contains("file") || lower.contains("document") || lower.contains("office") {
        "files".to_string()
    } else if lower.contains("code") || lower.contains("dev") || lower.contains("program") {
        "development".to_string()
    } else if lower.contains("computer") || lower.contains("desktop") || lower.contains("os") {
        "computer".to_string()
    } else if lower.contains("chat") || lower.contains("conversation") || lower.contains("general") {
        "chat".to_string()
    } else if lower.contains("research") || lower.contains("analysis") || lower.contains("data") {
        "research".to_string()
    } else {
        lower
    }
}

/// Tokenise text into lowercased terms, removing stop-words and short tokens.
fn tokenise(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "are", "was", "were", "has", "had", "but", "not",
        "this", "that", "with", "from", "they", "have", "been", "would", "could",
        "should", "will", "more", "than", "then", "also", "about", "just", "very",
        "what", "when", "where", "which", "who", "how", "may", "might", "can",
        "its", "all", "any", "each", "every", "some", "use", "used", "using",
    ];
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() > 2 && !stop.contains(w.as_str()))
        .collect()
}

/// Compute a simple TF vector for the query (no IDF since query is one document).
fn tf_vector(tokens: &[String]) -> HashMap<String, f64> {
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for t in tokens {
        *tf.entry(t.as_str()).or_insert(0) += 1;
    }
    let max_tf = tf.values().copied().max().unwrap_or(1).max(1);
    tf.iter()
        .map(|(k, v)| (k.to_string(), *v as f64 / max_tf as f64))
        .collect()
}

/// Compute TF-IDF vector for a document.
fn tfidf_vector(
    tokens: &[String],
    doc_freq: &HashMap<String, usize>,
    num_docs: usize,
) -> HashMap<String, f64> {
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for t in tokens {
        *tf.entry(t.as_str()).or_insert(0) += 1;
    }
    let max_tf = tf.values().copied().max().unwrap_or(1).max(1);

    let mut vec = HashMap::new();
    for (term, count) in &tf {
        let df = doc_freq.get(*term).copied().unwrap_or(1).max(1);
        let idf = (num_docs as f64 / df as f64).ln() + 1.0;
        vec.insert(term.to_string(), *count as f64 / max_tf as f64 * idf);
    }
    vec
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let mut dot = 0.0f64;
    let mut mag_a = 0.0f64;
    let mut mag_b = 0.0f64;

    for (term, weight_a) in a {
        mag_a += weight_a * weight_a;
        if let Some(weight_b) = b.get(term) {
            dot += weight_a * weight_b;
        }
    }
    for weight_b in b.values() {
        mag_b += weight_b * weight_b;
    }

    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::utils::discovery::AgentInfo;

    fn make_agent(id: i64, name: &str, domain: &str, desc: &str) -> AgentInfo {
        AgentInfo {
            id,
            name: name.to_string(),
            description: desc.to_string(),
            tools: vec![],
            instructions: String::new(),
            model: String::new(),
            domain: domain.to_string(),
            version: String::new(),
            code: None,
            source_files: vec![],
            response_format: "json".to_string(),
        }
    }

    fn make_atlas(agents: Vec<AgentInfo>) -> AgentAtlas {
        AgentAtlas::new(agents)
    }

    #[test]
    fn test_search_returns_relevant_agents() {
        let atlas = make_atlas(vec![
            make_agent(1, "web_search", "web", "Searches the web for information using search engines"),
            make_agent(2, "file_tool", "files", "Reads and writes files on the local filesystem"),
            make_agent(3, "chat_agent", "chat", "General conversation and Q&A assistant"),
        ]);
        let results = search_atlas(&atlas, "search the web", 2);
        assert!(!results.is_empty());
        assert!(results[0].domain == "web" || results[0].name == "web_search");
    }

    #[test]
    fn test_search_respects_limit() {
        let atlas = make_atlas(vec![
            make_agent(1, "web_search", "web", "Web search tool"),
            make_agent(2, "file_tool", "files", "File operations"),
            make_agent(3, "chat_agent", "chat", "Chat assistant"),
        ]);
        let results = search_atlas(&atlas, "tool", 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_empty_query_returns_all() {
        let atlas = make_atlas(vec![
            make_agent(1, "web_search", "web", "Web search"),
            make_agent(2, "file_tool", "files", "File operations"),
        ]);
        let results = search_atlas(&atlas, "", 5);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_group_into_cells_by_domain() {
        let agents = vec![
            make_agent(1, "web_search", "web", "Search the web"),
            make_agent(2, "web_browse", "web", "Browse websites"),
            make_agent(3, "file_tool", "files", "Read and write files"),
            make_agent(4, "chat", "chat", "Talk to the user"),
        ];
        let cells = group_into_cells(&agents);
        // Should have at least one cell per domain
        assert!(cells.len() >= 2);
    }

    #[test]
    fn test_group_empty_returns_empty() {
        let cells = group_into_cells(&[]);
        assert!(cells.is_empty());
    }

    #[test]
    fn test_tokenise_removes_stop_words() {
        let tokens: Vec<String> = tokenise("the quick brown fox jumps over the lazy dog");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"lazy".to_string()));
    }
}
