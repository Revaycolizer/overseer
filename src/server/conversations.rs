use super::ServerState;
use crate::backend::Conversation;
use axum::{
    extract::{Json, Query, State},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LimitQuery {
    pub limit: i64,
}

#[derive(Deserialize)]
pub struct AddConvPayload {
    pub title: String,
}

#[derive(Deserialize)]
pub struct UpdateTitlePayload {
    pub id: i64,
    pub title: String,
}

#[derive(Deserialize)]
pub struct DeleteConvPayload {
    pub id: i64,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route(
            "/api/conversations",
            get(get_conversations).post(add_conversation),
        )
        .route(
            "/api/conversations/update_title",
            post(update_conversation_title),
        )
        .route("/api/conversations/delete", post(delete_conversation))
        .route("/api/conversations/search", get(search_messages))
}

async fn get_conversations(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<LimitQuery>,
) -> Result<Json<Vec<Conversation>>, String> {
    crate::backend::get_conversations(&state.db, query.limit)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn add_conversation(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<AddConvPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_conversation(&state.db, &payload.title)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn update_conversation_title(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateTitlePayload>,
) -> Result<(), String> {
    crate::backend::update_conversation_title(&state.db, payload.id, &payload.title)
        .await
        .map_err(|e| e.to_string())
}

async fn delete_conversation(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<DeleteConvPayload>,
) -> Result<(), String> {
    crate::backend::delete_conversation(&state.db, payload.id)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

async fn search_messages(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<crate::backend::ConversationSearchResult>>, String> {
    crate::backend::search_messages(&state.db, &query.q)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}
