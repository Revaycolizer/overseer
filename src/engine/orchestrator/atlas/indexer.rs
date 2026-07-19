use crate::engine::utils::discovery::AgentInfo;

/// In-memory agent index with optional LanceDB persistent vector store.
///
/// The `agents` vector is the primary store. When the `memory` feature is
/// enabled, `build_index()` can sync embeddings to LanceDB for semantic search.
pub struct AgentAtlas {
    pub agents: Vec<AgentInfo>,
}

impl AgentAtlas {
    pub fn new(agents: Vec<AgentInfo>) -> Self {
        Self { agents }
    }

    pub fn get_by_domain(&self, domain: &str) -> Option<&AgentInfo> {
        self.agents.iter().find(|a| a.domain == domain)
    }

    pub fn get_all_domains(&self) -> Vec<String> {
        self.agents.iter().map(|a| a.domain.clone()).collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Build a LanceDB vector index from the current agent list.
    ///
    /// Requires the `memory` feature. Each agent's domain + description is
    /// embedded via Ollama (`nomic-embed-text`) and stored in a `agent_atlas`
    /// table. Idempotent: re-running replaces the table.
    #[cfg(feature = "memory")]
    pub async fn build_lancedb_index(
        &self,
        lancedb_uri: &str,
        client: &reqwest::Client,
    ) -> anyhow::Result<()> {
        use arrow::array::{Float32Array, Int64Array, StringArray, RecordBatch};
        use arrow::datatypes::{DataType, Field, Schema};
        use futures::stream;
        use std::sync::Arc;

        let db = lancedb::connect(lancedb_uri).execute().await?;

        // Build embedding vectors for all agents
        let text_for_embedding: Vec<&str> = self
            .agents
            .iter()
            .map(|a| {
                // Clear cache so we always get fresh text
                Box::leak(Box::new(format!("{}: {}", a.domain, a.description)))
            })
            .collect();

        let embed_results =
            crate::engine::utils::embeddings::embed_batch(client, &text_for_embedding).await;

        let mut ids = Vec::new();
        let mut vectors = Vec::new();
        let mut domains = Vec::new();
        let mut descriptions = Vec::new();

        for (i, agent) in self.agents.iter().enumerate() {
            let embedding = embed_results.as_ref().and_then(|v| v.get(i).cloned());
            if let Some(vec) = embedding {
                if vec.len() == crate::engine::utils::embeddings::EMBEDDING_DIM {
                    ids.push(agent.id);
                    vectors.extend(vec);
                    domains.push(agent.domain.clone());
                    descriptions.push(agent.description.clone());
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
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    crate::engine::utils::embeddings::EMBEDDING_DIM as i32,
                ),
                false,
            ),
            Field::new("domain", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, false),
        ]));

        let dim = crate::engine::utils::embeddings::EMBEDDING_DIM;
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(
                    arrow::array::FixedSizeListArray::from_iter_primitive::<Float32Array, _, _>(
                        vectors.chunks(dim).map(|c| Some(c.iter().map(|&x| Some(x)))),
                        dim as i32,
                    ),
                ),
                Arc::new(StringArray::from(domains)),
                Arc::new(StringArray::from(descriptions)),
            ],
        )?;

        // Drop old table if exists, create fresh
        let _ = db.drop_table("agent_atlas").execute().await;
        let table = db
            .create_table(
                "agent_atlas",
                Box::new(stream::once(async move { Ok(batch.clone()) })),
            )
            .execute()
            .await?;

        tracing::info!(
            "AgentAtlas: indexed {} agents into LanceDB table 'agent_atlas'",
            ids.len()
        );
        let _ = table;
        Ok(())
    }
}

/// Serialise an agent for embedding (domain + description).
pub fn vectorize_agent(agent: &AgentInfo) -> String {
    format!("{}: {}", agent.domain, agent.description)
}
