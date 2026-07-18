use axum::{
    routing::{get, post},
    extract::{State, Json},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use super::ServerState;

#[derive(Deserialize)]
struct RegisterPayload { username: String, display_name: String, password: String }

#[derive(Deserialize)]
struct LoginPayload { username: String, password: String }

#[derive(Serialize)]
struct AuthResponse { token: String, user_id: i64 }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
}

async fn register(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<AuthResponse>, String> {
    let hash = crate::backend::auth::hash_password(&payload.password).map_err(|e| e.to_string())?;
    let id = state.db.next_id().await;
    sqlx::query("INSERT INTO users (id, username, display_name, password_hash) VALUES (?, ?, ?, ?)")
        .bind(id).bind(&payload.username).bind(&payload.display_name).bind(&hash)
        .execute(&state.db.pool).await.map_err(|e| e.to_string())?;
    let token = crate::backend::auth::create_session(&state.db, id).await.map_err(|e| e.to_string())?;
    Ok(Json(AuthResponse { token, user_id: id }))
}

async fn login(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, String> {
    let user = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, password_hash FROM users WHERE username = ? AND is_active = 1",
    )
    .bind(&payload.username)
    .fetch_optional(&state.db.pool)
    .await.map_err(|e| e.to_string())?
    .ok_or_else(|| "Invalid credentials".to_string())?;

    let valid = crate::backend::auth::verify_password(&payload.password, &user.1)
        .map_err(|e| e.to_string())?;
    if !valid { return Err("Invalid credentials".to_string()); }

    let token = crate::backend::auth::create_session(&state.db, user.0).await.map_err(|e| e.to_string())?;
    Ok(Json(AuthResponse { token, user_id: user.0 }))
}
