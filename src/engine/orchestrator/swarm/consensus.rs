use std::collections::{BTreeMap, HashMap, HashSet};

/// Calculate consensus across agent outputs using TF-IDF weighted similarity.
///
/// Each output is tokenised into terms (lowercased, stop-words removed).
/// Term frequency–inverse document frequency (TF-IDF) weights are computed
/// across the corpus of outputs. The output with the highest mean cosine
/// similarity to all others (under the TF-IDF weighting) is selected as the
/// consensus. This is strictly better than raw keyword-overlap because it
/// down-weights common words and up-weights distinctive terms.
pub fn calculate_consensus(outputs: &[String]) -> Option<String> {
    if outputs.is_empty() {
        return None;
    }
    if outputs.len() == 1 {
        return Some(outputs[0].clone());
    }

    // 1. Tokenise and build document-frequency map
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let tokenised: Vec<Vec<String>> = outputs
        .iter()
        .map(|text| tokenise(text))
        .collect();

    let num_docs = outputs.len();
    for tokens in &tokenised {
        let unique: HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
        for term in unique {
            *doc_freq.entry(term.to_string()).or_insert(0) += 1;
        }
    }

    // 2. Compute TF-IDF vectors
    let vectors: Vec<BTreeMap<String, f64>> = tokenised
        .iter()
        .map(|tokens| tfidf_vector(tokens, &doc_freq, num_docs))
        .collect();

    // 3. Find the output with highest mean cosine similarity to all others
    let mut best_idx = 0usize;
    let mut best_mean_sim = -1.0f64;

    for i in 0..vectors.len() {
        let mut total_sim = 0.0f64;
        let mut count = 0usize;
        for j in 0..vectors.len() {
            if i == j {
                continue;
            }
            total_sim += cosine_similarity(&vectors[i], &vectors[j]);
            count += 1;
        }
        if count > 0 {
            let mean = total_sim / count as f64;
            if mean > best_mean_sim {
                best_mean_sim = mean;
                best_idx = i;
            }
        }
    }

    Some(outputs[best_idx].clone())
}

/// Tokenise text into lowercased terms, removing stop-words and short tokens.
fn tokenise(text: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "by", "with", "from", "as", "is", "was", "are", "were", "be",
        "been", "being", "have", "has", "had", "do", "does", "did", "will",
        "would", "could", "should", "may", "might", "shall", "can", "it",
        "its", "this", "that", "these", "those", "i", "we", "you", "he",
        "she", "they", "me", "him", "her", "us", "them", "my", "your",
        "his", "her", "our", "their", "not", "no", "nor", "so", "if",
        "then", "than", "too", "very", "just", "about", "also", "more",
    ];
    let stop_set: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|w| w.len() > 2 && !stop_set.contains(w.as_str()))
        .collect()
}

/// Compute the TF-IDF vector for a single document's tokens.
fn tfidf_vector(
    tokens: &[String],
    doc_freq: &HashMap<String, usize>,
    num_docs: usize,
) -> BTreeMap<String, f64> {
    let mut tf: HashMap<&str, usize> = HashMap::new();
    for t in tokens {
        *tf.entry(t.as_str()).or_insert(0) += 1;
    }

    let max_tf = tf.values().copied().max().unwrap_or(1).max(1);

    let mut vec = BTreeMap::new();
    for (term, count) in &tf {
        let term_str = term.to_string();
        let df = *doc_freq.get(&term_str).unwrap_or(&1).max(&1);
        let idf = (num_docs as f64 / df as f64).ln() + 1.0;
        let tf_norm = *count as f64 / max_tf as f64;
        vec.insert(term_str, tf_norm * idf);
    }
    vec
}

/// Cosine similarity between two TF-IDF vectors.
fn cosine_similarity(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>) -> f64 {
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

    #[test]
    fn test_consensus_single_output() {
        let outputs = vec!["Only one agent response.".to_string()];
        let result = calculate_consensus(&outputs);
        assert_eq!(result, Some("Only one agent response.".to_string()));
    }

    #[test]
    fn test_consensus_empty() {
        let outputs: Vec<String> = vec![];
        assert_eq!(calculate_consensus(&outputs), None);
    }

    #[test]
    fn test_consensus_selects_majority() {
        let outputs = vec![
            "Quantum computing uses superposition and entanglement.".to_string(),
            "Quantum computing uses superposition and entanglement for computation.".to_string(),
            "The weather today is sunny with a chance of rain.".to_string(),
        ];
        let result = calculate_consensus(&outputs).unwrap();
        assert!(
            result.contains("Quantum") || result.contains("quantum"),
            "Expected quantum-related consensus, got: {}",
            result
        );
    }

    #[test]
    fn test_tokenise_removes_stop_words() {
        let tokens = tokenise("The quick brown fox jumps over the lazy dog");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"over".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"lazy".to_string()));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut v = BTreeMap::new();
        v.insert("hello".to_string(), 1.0);
        v.insert("world".to_string(), 2.0);
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut v1 = BTreeMap::new();
        v1.insert("hello".to_string(), 1.0);
        let v2 = BTreeMap::new();
        let sim = cosine_similarity(&v1, &v2);
        assert!((sim - 0.0).abs() < 1e-10);
    }
}
