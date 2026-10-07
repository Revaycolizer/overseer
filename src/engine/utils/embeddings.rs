/// Shared embedding utilities using Ollama's `/api/embed` endpoint.
///
/// Uses `nomic-embed-text` model by default (768-dimensional vectors).
/// All functions are async and fallible — if Ollama is unreachable they
/// return `None` so callers can fall back to keyword-based methods.
use serde_json::Value;

/// Default embedding model.
pub const EMBEDDING_MODEL: &str = "nomic-embed-text";

/// Embedding dimension for `nomic-embed-text`.
pub const EMBEDDING_DIM: usize = 768;

/// Embed a single text string into a vector of f32.
///
/// Returns `None` if Ollama is unreachable or returns an error.
pub async fn embed_text(client: &reqwest::Client, text: &str) -> Option<Vec<f32>> {
    let resp = client
        .post("http://localhost:11434/api/embed")
        .json(&serde_json::json!({
            "model": EMBEDDING_MODEL,
            "input": text,
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: Value = resp.json().await.ok()?;
    json.get("embeddings")
        .and_then(|e| e.as_array())
        .and_then(|e| e.first())
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        })
}

/// Embed multiple texts in a single API call (batch).
///
/// Returns a vector of embeddings in the same order as `texts`.
/// If Ollama is unreachable, returns `None`.
pub async fn embed_batch(client: &reqwest::Client, texts: &[&str]) -> Option<Vec<Vec<f32>>> {
    let resp = client
        .post("http://localhost:11434/api/embed")
        .json(&serde_json::json!({
            "model": EMBEDDING_MODEL,
            "input": texts,
        }))
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: Value = resp.json().await.ok()?;
    json.get("embeddings")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| {
                    item.as_array()
                        .map(|v| {
                            v.iter()
                                .filter_map(|x| x.as_f64().map(|f| f as f32))
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let denom = mag_a * mag_b;
    if denom < 1e-10 {
        0.0
    } else {
        (dot / denom) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }
}
