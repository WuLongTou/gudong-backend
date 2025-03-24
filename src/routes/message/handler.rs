use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::model::{CreateMessageRequest, GetMessagesRequest, MessageInfo};
use crate::AppState;
use crate::utils::{Claims, error_codes, error_to_api_response, success_to_api_response};

#[axum::debug_handler]
pub async fn create_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateMessageRequest>,
) -> impl IntoResponse {
    match MessageInfo::create(&state.pool, &state.redis, req, claims.sub).await {
        Ok(message) => (
            StatusCode::CREATED,
            success_to_api_response(serde_json::json!({
                "message_id": message.message_id
            })),
        ),
        Err(e) => {
            tracing::error!("Failed to create message: {}", e);
            let status = if e.to_string().contains("User is not a member") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn get_messages(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<GetMessagesRequest>,
) -> impl IntoResponse {
    match MessageInfo::get_messages(&state.pool, &state.redis, req, &claims.sub).await {
        Ok(messages) => (StatusCode::OK, success_to_api_response(messages)),
        Err(e) => {
            tracing::error!("Failed to get messages: {}", e);
            let status = if e.to_string().contains("User is not a member") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}
