use axum::{
    routing::{get, post},
    extract::{State, Json, Path},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    body::Bytes,
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
pub struct UserProfile { pub id: i64, pub username: String, pub display_name: String }

#[derive(Serialize)]
struct AuthResponse { token: String, user: UserProfile }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/auth/register", post(register))
        .route("/v1/auth/login", post(login))
        .route("/v1/auth/logout", post(logout))
        .route("/v1/auth/me", get(me))
        .route("/v1/users/{id}/avatar", get(get_avatar))
}

async fn get_user_from_token(state: &Arc<ServerState>, headers: &HeaderMap) -> Result<UserProfile, StatusCode> {
    let token = headers.get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT u.id, u.username, u.display_name FROM user_sessions s
         JOIN users u ON u.id = s.user_id
         WHERE s.token = ? AND s.expires_at > datetime('now') AND u.is_active = 1",
    )
    .bind(token)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map(|(id, un, dn)| UserProfile { id, username: un, display_name: dn })
    .ok_or(StatusCode::UNAUTHORIZED)
}

async fn register(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<RegisterPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let hash = crate::backend::auth::hash_password(&payload.password).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = state.db.next_id().await;
    sqlx::query("INSERT INTO users (id, username, display_name, password_hash) VALUES (?, ?, ?, ?)")
        .bind(id).bind(&payload.username).bind(&payload.display_name).bind(&hash)
        .execute(&state.db.pool).await.map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    let token = crate::backend::auth::create_session(&state.db, id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(AuthResponse { token, user: UserProfile { id, username: payload.username, display_name: payload.display_name } }))
}

async fn login(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, password_hash, display_name FROM users WHERE username = ? AND is_active = 1",
    )
    .bind(&payload.username)
    .fetch_optional(&state.db.pool)
    .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid credentials".into()))?;

    let valid = crate::backend::auth::verify_password(&payload.password, &user.1).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !valid { return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".into())); }

    let token = crate::backend::auth::create_session(&state.db, user.0).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(AuthResponse { token, user: UserProfile { id: user.0, username: payload.username, display_name: user.2 } }))
}

async fn logout(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> StatusCode {
    if let Some(token) = headers.get("Authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")) {
        let _ = sqlx::query("DELETE FROM user_sessions WHERE token = ?").bind(token).execute(&state.db.pool).await;
    }
    StatusCode::NO_CONTENT
}

async fn me(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Result<Json<UserProfile>, StatusCode> {
    let user = get_user_from_token(&state, &headers).await?;
    Ok(Json(user))
}

async fn get_avatar(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let data = sqlx::query_scalar::<_, Vec<u8>>("SELECT avatar_data FROM users WHERE id = ?")
        .bind(id).fetch_optional(&state.db.pool).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .and_then(|v| if v.is_empty() { None } else { Some(v) })
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(([(header::CONTENT_TYPE, "image/png")], Bytes::from(data)))
}
