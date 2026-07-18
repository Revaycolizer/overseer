use super::{CellState, GlobalState};

pub fn sync_cell_to_global(cell: &CellState, global: &mut GlobalState) {
    // Only push facts that are explicitly verified or consensus-backed
    // For now, we sync all facts from the cell (Phase 1)
    for fact in &cell.facts {
        global.commit_fact(fact.clone());
    }
    
    for source in &cell.sources {
        global.commit_source(source.clone());
    }
    
    tracing::debug!(cell_id = %cell.cell_id, "LATTICE synced cell to global state");
}

pub fn filter_relevant_facts(global: &GlobalState, query: &str) -> Vec<String> {
    // Simple keyword-based filtering for now to keep under 100 lines.
    // V3.1 will use vector similarity.
    let lower_query = query.to_lowercase();
    global.verified_facts
        .iter()
        .filter(|f| {
            let lower_f = f.to_lowercase();
            lower_query.split_whitespace().any(|word| word.len() > 3 && lower_f.contains(word))
        })
        .cloned()
        .collect()
}
