// src/db/vector_sync.rs — #[cfg(feature = "memory")] vector DB sync
use super::Database;
use arrow::array::{Array, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatchIterator;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
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
                let embed_resp = client
                    .post("http://localhost:11434/api/embed")
                    .json(&serde_json::json!({
                        "model": "nomic-embed-text",
                        "input": desc
                    }))
                    .send()
                    .await?;

                let json: serde_json::Value = embed_resp.json().await?;
                if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
                    if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                        let vec: Vec<f32> = first
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        if vec.len() != 768 || vec.iter().any(|value| !value.is_finite()) {
                            continue;
                        }
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
            Field::new(
                "vector",
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 768),
                false,
            ),
            Field::new("domain", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(arrow::array::FixedSizeListArray::from_iter_primitive::<
                    arrow::datatypes::Float32Type,
                    _,
                    _,
                >(
                    vectors
                        .chunks(768)
                        .map(|c| Some(c.iter().map(|&x| Some(x)))),
                    768,
                )),
                Arc::new(StringArray::from(domains)),
                Arc::new(StringArray::from(descriptions)),
            ],
        )?;

        match db.open_table("agents").execute().await {
            Ok(table) => {
                table
                    .add(Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema))
                        as Box<dyn arrow::record_batch::RecordBatchReader + Send>)
                    .execute()
                    .await?;
            }
            Err(_) => {
                db.create_table(
                    "agents",
                    Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema))
                        as Box<dyn arrow::record_batch::RecordBatchReader + Send>,
                )
                .execute()
                .await?;
            }
        }

        Ok(())
    }

    pub async fn search_agents_semantically(
        &self,
        client: &reqwest::Client,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<String>> {
        let db = lancedb::connect(&self.lancedb_uri).execute().await?;
        let table = db.open_table("agents").execute().await?;

        // Embed query
        let embed_resp = client
            .post("http://localhost:11434/api/embed")
            .json(&serde_json::json!({
                "model": "nomic-embed-text",
                "input": query
            }))
            .send()
            .await?;

        let json: serde_json::Value = embed_resp.json().await?;
        let embedding = if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array())
        {
            if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                first
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect::<Vec<f32>>()
            } else {
                return Err(anyhow::anyhow!("No embeddings returned"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to parse embeddings"));
        };

        let mut batches = table
            .query()
            .nearest_to(embedding)?
            .limit(limit)
            .execute()
            .await?;

        let mut domains = Vec::new();
        while let Some(batch) = batches.try_next().await? {
            let domain_col = batch
                .column_by_name("domain")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| anyhow::anyhow!("Domain column missing"))?;

            for i in 0..domain_col.len() {
                domains.push(domain_col.value(i).to_string());
            }
        }
        Ok(domains)
    }
}
