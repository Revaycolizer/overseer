use super::ServerState;
use crate::backend::{Attachment, Message};
use axum::{
    extract::{Json, Query, State},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ConversationQuery {
    pub conversation_id: i64,
}

#[derive(Deserialize)]
pub struct MessageQuery {
    pub message_id: i64,
}

#[derive(Deserialize)]
pub struct AddMsgPayload {
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub hide_evidence: bool,
}

#[derive(Deserialize)]
pub struct AddAttachmentPayload {
    pub message_id: i64,
    pub filename: String,
    pub file_path: String,
    pub content_type: String,
    pub size: i64,
}

#[derive(Deserialize)]
pub struct UpdateAttachmentPayload {
    pub id: i64,
    pub content: String,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/messages", get(get_messages).post(add_message))
        .route("/api/messages/delete", post(delete_message))
        .route(
            "/api/attachments",
            get(get_attachments).post(add_attachment),
        )
        .route(
            "/api/attachments/update_content",
            post(update_attachment_content),
        )
}

async fn get_messages(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<ConversationQuery>,
) -> Result<Json<Vec<Message>>, String> {
    crate::backend::get_messages(&state.db, query.conversation_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn add_message(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<AddMsgPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_message(
        &state.db,
        payload.conversation_id,
        &payload.role,
        &payload.content,
        payload.hide_evidence,
    )
    .await
    .map(Json)
    .map_err(|e| e.to_string())
}

async fn get_attachments(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Vec<Attachment>>, String> {
    crate::backend::get_attachments(&state.db, query.message_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn add_attachment(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<AddAttachmentPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_attachment(
        &state.db,
        payload.message_id,
        &payload.filename,
        &payload.file_path,
        &payload.content_type,
        payload.size,
    )
    .await
    .map(Json)
    .map_err(|e| e.to_string())
}

async fn update_attachment_content(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<UpdateAttachmentPayload>,
) -> Result<(), String> {
    crate::backend::update_attachment_content(&state.db, payload.id, &payload.content)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct DeleteMessagePayload {
    pub id: i64,
}

async fn delete_message(
    State(state): State<Arc<ServerState>>,
    Json(payload): Json<DeleteMessagePayload>,
) -> Result<(), String> {
    crate::backend::delete_message(&state.db, payload.id)
        .await
        .map_err(|e| e.to_string())
}
