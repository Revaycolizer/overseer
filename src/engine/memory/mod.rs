use arrow::array::{Array, ArrayRef, FixedSizeListArray, Float32Array, StringArray};
use arrow::datatypes::{DataType, Field, FieldRef, Schema};
use arrow::record_batch::RecordBatch;
use arrow::record_batch::RecordBatchIterator;
use futures::TryStreamExt;
use lancedb::query::*;
use rig::completion::Message;
use rig::providers::ollama;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemoryRow {
    pub id: String,
    pub content: String,
    pub vector: Vec<f32>,
}

pub const EMBEDDING_DIM: usize = 768; // nomic-embed-text dimension

pub async fn get_embedding(_client: &ollama::Client, text: &str) -> Option<Vec<f32>> {
    // Use ollama embeddings via reqwest since rig-core may not expose embedding API directly
    let body = serde_json::json!({
        "model": "nomic-embed-text",
        "input": text
    });

    match reqwest::Client::new()
        .post("http://localhost:11434/api/embed")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
                    if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                        let vec: Vec<f32> = first
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        if vec.len() == EMBEDDING_DIM {
                            return Some(vec);
                        }
                    }
                }
            }
            None
        }
        Err(e) => {
            eprintln!("⚠️ Embedding request failed: {}", e);
            None
        }
    }
}

pub async fn setup_memory_table(db: &lancedb::Connection) -> Option<lancedb::Table> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM as i32,
            ),
            false,
        ),
    ]));

    // Try to open existing table first
    if let Ok(table) = db.open_table("memories").execute().await {
        println!("📚 Memory table loaded.");
        return Some(table);
    }

    // Create new table with empty batch
    let ids = StringArray::from(vec!["init"]);
    let contents = StringArray::from(vec!["System initialized"]);
    let zero_vec: Vec<f32> = vec![0.0; EMBEDDING_DIM];
    let values = Float32Array::from(zero_vec);
    let field_ref: FieldRef = Arc::new(Field::new("item", DataType::Float32, true)).into();
    let list = FixedSizeListArray::new(
        field_ref,
        EMBEDDING_DIM as i32,
        Arc::new(values) as ArrayRef,
        None,
    );

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ids) as ArrayRef,
            Arc::new(contents) as ArrayRef,
            Arc::new(list) as ArrayRef,
        ],
    )
    .ok()?;

    let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
    match db
        .create_table("memories", Box::new(batches))
        .execute()
        .await
    {
        Ok(table) => {
            println!("📚 Memory table created.");
            Some(table)
        }
        Err(e) => {
            eprintln!("⚠️ Failed to create memory table: {}", e);
            None
        }
    }
}

pub async fn store_memory(
    table: &lancedb::Table,
    client: &ollama::Client,
    query: &str,
    result: &str,
) {
    let content = format!("User asked: {}\nAssistant replied: {}", query, result);
    let embedding = match get_embedding(client, &content).await {
        Some(e) => e,
        None => {
            eprintln!("⚠️ Could not generate embedding, skipping memory storage.");
            return;
        }
    };

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM as i32,
            ),
            false,
        ),
    ]));

    let id = uuid::Uuid::new_v4().to_string();
    let ids = StringArray::from(vec![id.as_str()]);
    let contents = StringArray::from(vec![content.as_str()]);
    let values = Float32Array::from(embedding);
    let field_ref: FieldRef = Arc::new(Field::new("item", DataType::Float32, true)).into();
    let list = FixedSizeListArray::new(
        field_ref,
        EMBEDDING_DIM as i32,
        Arc::new(values) as ArrayRef,
        None,
    );

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(ids) as ArrayRef,
            Arc::new(contents) as ArrayRef,
            Arc::new(list) as ArrayRef,
        ],
    );

    match batch {
        Ok(b) => {
            let batches = RecordBatchIterator::new(vec![Ok(b)], schema);
            if let Err(e) = table.add(Box::new(batches)).execute().await {
                eprintln!("⚠️ Failed to store memory: {}", e);
            } else {
                println!("💾 Memory stored successfully.");
            }
        }
        Err(e) => eprintln!("⚠️ Failed to create memory batch: {}", e),
    }
}

pub async fn retrieve_memories(
    table: &lancedb::Table,
    client: &ollama::Client,
    query: &str,
    limit: usize,
) -> Vec<String> {
    let embedding = match get_embedding(client, query).await {
        Some(e) => e,
        None => return vec![],
    };

    match table
        .vector_search(embedding)
        .expect("Failed to create vector search")
        .limit(limit)
        .execute()
        .await
    {
        Ok(mut results) => {
            let mut memories = Vec::new();
            while let Ok(Some(batch)) = results.try_next().await {
                if let Some(content_col) = batch.column_by_name("content") {
                    if let Some(arr) = content_col.as_any().downcast_ref::<StringArray>() {
                        for i in 0..arr.len() {
                            if !arr.is_null(i) {
                                let val = arr.value(i);
                                if val != "System initialized" {
                                    memories.push(val.to_string());
                                }
                            }
                        }
                    }
                }
            }
            memories
        }
        Err(e) => {
            eprintln!("⚠️ Memory search failed: {}", e);
            vec![]
        }
    }
}

#[allow(dead_code)]
pub fn parse_memory_to_messages(content: &str) -> Vec<Message> {
    if let Some((user_part, assistant_part)) = content.split_once("\nAssistant replied: ") {
        let user_text = user_part.trim_start_matches("User asked: ").trim();
        let assistant_text = assistant_part.trim();
        if user_text.is_empty() {
            return vec![];
        }
        let mut msgs = vec![Message::User {
            content: rig::OneOrMany::one(rig::message::UserContent::text(user_text.to_string())),
        }];
        if !assistant_text.is_empty() {
            msgs.push(Message::Assistant {
                id: None,
                content: rig::OneOrMany::one(rig::message::AssistantContent::text(
                    assistant_text.to_string(),
                )),
            });
        }
        msgs
    } else {
        vec![]
    }
}
