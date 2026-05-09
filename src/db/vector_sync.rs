// src/db/vector_sync.rs — #[cfg(feature = "memory")] vector DB sync
use super::Database;
use arrow::array::{Float32Array, Int64Array, StringArray, RecordBatch};
use arrow::datatypes::{DataType, Field, Schema};
use std::sync::Arc;

impl Database {
    pub async fn sync_agents_to_vector_db(&self, client: &reqwest::Client) -> anyhow::Result<()> {
        let db = lancedb::connect(&self.lancedb_uri).execute().await?;
        
        let agents: Vec<crate::db::Supervisor> = sqlx::query_as("SELECT * FROM supervisors")
            .fetch_all(&self.pool)
            .await?;

        let mut ids = Vec::new();
        let mut vectors = Vec::new();
        let mut domains = Vec::new();
        let mut descriptions = Vec::new();

        for agent in agents {
            if let Some(desc) = agent.description {
                // Call Ollama embeddings API
                let embed_resp = client.post("http://localhost:11434/api/embed")
                    .json(&serde_json::json!({
                        "model": "nomic-embed-text",
                        "input": desc
                    }))
                    .send()
                    .await?;
                
                let json: serde_json::Value = embed_resp.json().await?;
                if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
                    if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                        let vec: Vec<f32> = first.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
                        ids.push(agent.id);
                        vectors.extend(vec);
                        domains.push(agent.domain);
                        descriptions.push(desc);
                    }
                }
            }
        }

        if ids.is_empty() {
            return Ok(());
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("vector", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 768), false),
            Field::new("domain", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(arrow::array::FixedSizeListArray::from_iter_primitive::<Float32Array, _, _>(
                    vectors.chunks(768).map(|c| Some(c.iter().map(|&x| Some(x)))),
                    768,
                )),
                Arc::new(StringArray::from(domains)),
                Arc::new(StringArray::from(descriptions)),
            ],
        )?;

        let table = db.open_table("agents").execute().await;
        let table = match table {
            Ok(t) => t,
            Err(_) => db.create_table("agents", Box::new(futures::stream::once(async move { Ok(batch.clone()) }))).execute().await?,
        };

        table.add(Box::new(futures::stream::once(async move { Ok(batch) }))).execute().await?;

        Ok(())
    }

    pub async fn search_agents_semantically(&self, client: &reqwest::Client, query: &str, limit: usize) -> anyhow::Result<Vec<String>> {
        let db = lancedb::connect(&self.lancedb_uri).execute().await?;
        let table = db.open_table("agents").execute().await?;

        // Embed query
        let embed_resp = client.post("http://localhost:11434/api/embed")
            .json(&serde_json::json!({
                "model": "nomic-embed-text",
                "input": query
            }))
            .send()
            .await?;
        
        let json: serde_json::Value = embed_resp.json().await?;
        let embedding = if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
            if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                first.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect::<Vec<f32>>()
            } else {
                return Err(anyhow::anyhow!("No embeddings returned"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to parse embeddings"));
        };

        let batches = table
            .query()
            .nearest_to(embedding)?
            .limit(limit)
            .execute()
            .await?;

        let mut domains = Vec::new();
        for batch in batches {
            let domain_col = batch.column_by_name("domain")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| anyhow::anyhow!("Domain column missing"))?;
            
            for i in 0..domain_col.len() {
                domains.push(domain_col.value(i).to_string());
            }
        }
        Ok(domains)
    }
}
