pub fn calculate_consensus(outputs: &[String]) -> Option<String> {
    if outputs.is_empty() { return None; }
    
    // V3 Strategy: Group similar outputs and select the majority
    // Simplified: longest response that contains shared keywords
    let mut best_response = &outputs[0];
    let mut max_score = 0;
    
    for (i, output) in outputs.iter().enumerate() {
        let mut score = 0;
        for (j, other) in outputs.iter().enumerate() {
            if i == j { continue; }
            if shared_keywords(output, other) > 5 {
                score += 1;
            }
        }
        if score > max_score {
            max_score = score;
            best_response = output;
        }
    }
    
    Some(best_response.clone())
}

fn shared_keywords(a: &str, b: &str) -> usize {
    let a_words: std::collections::HashSet<_> = a.split_whitespace().collect();
    let b_words: std::collections::HashSet<_> = b.split_whitespace().collect();
    a_words.intersection(&b_words).count()
}
