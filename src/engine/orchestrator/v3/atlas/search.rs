use super::indexer::AgentAtlas;
use crate::engine::utils::discovery::AgentInfo;

pub fn search_atlas(atlas: &AgentAtlas, query: &str, limit: usize) -> Vec<AgentInfo> {
    let lower_query = query.to_lowercase();
    
    // V3 Strategy: Keyword overlap + Priority fallback
    let mut candidates: Vec<(AgentInfo, usize)> = atlas.agents.iter()
        .map(|a| {
            let desc = a.description.to_lowercase();
            let score = lower_query.split_whitespace()
                .filter(|word| word.len() > 3 && desc.contains(word))
                .count();
            (a.clone(), score)
        })
        .collect();

    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    
    candidates.into_iter()
        .take(limit)
        .map(|(a, _)| a)
        .collect()
}

pub fn group_into_cells(agents: &[AgentInfo]) -> Vec<Vec<AgentInfo>> {
    // Logic to group agents by domain or similarity to form a Swarm Cell
    let mut cells = Vec::new();
    if !agents.is_empty() {
        cells.push(agents.to_vec()); // For now, all top agents form one cell
    }
    cells
}
