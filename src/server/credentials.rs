use axum::{
    routing::{get, post, patch, delete},
    extract::{State, Json, Path},
    http::StatusCode,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use super::ServerState;

#[derive(Serialize)]
struct Credential { id: i64, name: String, cred_type: String }

#[derive(Deserialize)]
struct CreatePayload { name: String, value: String, cred_type: Option<String> }

#[derive(Deserialize)]
struct UpdatePayload { name: Option<String>, value: Option<String> }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/credentials", get(list).post(create))
        .route("/v1/credentials/{id}", patch(update).delete(delete_cred))
}

async fn list(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Credential>>, StatusCode> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, cred_type FROM credentials ORDER BY created_at DESC",
    )
    .fetch_all(&state.db.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(|(id, name, cred_type)| Credential { id, name, cred_type }).collect()))
}

async fn create(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<Credential>, (StatusCode, String)> {
    let encrypted = crate::engine::workflow::credentials::encrypt(&payload.value)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = state.db.next_id().await;
    let ct = payload.cred_type.unwrap_or_else(|| "api_key".into());
    sqlx::query("INSERT INTO credentials (id, name, cred_type, encrypted_value, owner_id) VALUES (?, ?, ?, ?, 1)")
        .bind(id).bind(&payload.name).bind(&ct).bind(&encrypted)
        .execute(&state.db.pool).await.map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    Ok(Json(Credential { id, name: payload.name, cred_type: ct }))
}

async fn update(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<()>, StatusCode> {
    if let Some(name) = &payload.name {
        let _ = sqlx::query("UPDATE credentials SET name = ? WHERE id = ?").bind(name).bind(id).execute(&state.db.pool).await;
    }
    if let Some(value) = &payload.value {
        if let Ok(encrypted) = crate::engine::workflow::credentials::encrypt(value) {
            let _ = sqlx::query("UPDATE credentials SET encrypted_value = ? WHERE id = ?").bind(&encrypted).bind(id).execute(&state.db.pool).await;
        }
    }
    Ok(Json(()))
}

async fn delete_cred(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> StatusCode {
    let _ = sqlx::query("DELETE FROM credentials WHERE id = ?").bind(id).execute(&state.db.pool).await;
    StatusCode::NO_CONTENT
}
