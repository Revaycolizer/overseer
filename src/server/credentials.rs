use super::ServerState;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
struct Credential {
    id: i64,
    name: String,
    cred_type: String,
}

#[derive(Deserialize)]
struct CreatePayload {
    name: String,
    value: String,
    cred_type: Option<String>,
}

#[derive(Deserialize)]
struct UpdatePayload {
    name: Option<String>,
    value: Option<String>,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/credentials", get(list).post(create))
        .route("/v1/credentials/:id", patch(update).delete(delete_cred))
}

async fn list(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Credential>>, StatusCode> {
    let rows = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, name, cred_type FROM credentials ORDER BY created_at DESC",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|(id, name, cred_type)| Credential {
                id,
                name,
                cred_type,
            })
            .collect(),
    ))
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
    Ok(Json(Credential {
        id,
        name: payload.name,
        cred_type: ct,
    }))
}

async fn update(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<()>, StatusCode> {
    let encrypted = payload
        .value
        .as_deref()
        .map(crate::engine::workflow::credentials::encrypt)
        .transpose()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = sqlx::query("UPDATE credentials SET name=COALESCE(?,name), encrypted_value=COALESCE(?,encrypted_value) WHERE id=?")
        .bind(payload.name).bind(encrypted).bind(id).execute(&state.db.pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(()))
}

async fn delete_cred(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> StatusCode {
    match sqlx::query("DELETE FROM credentials WHERE id = ?")
        .bind(id)
        .execute(&state.db.pool)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT,
        Ok(_) => StatusCode::NOT_FOUND,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
