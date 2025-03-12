use axum::{
    Extension,
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};

use super::model::{CreateMessageRequest, CreateMessageResponse, GetMessagesRequest, MessageInfo};
use crate::utils::{error_to_api_response, success_to_api_response};
use crate::{AppState, utils::Claims};

#[axum::debug_handler]
pub async fn create_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateMessageRequest>,
) -> impl IntoResponse {
    match MessageInfo::create(&state.pool, req, claims.sub).await {
        Ok(_) => (
            StatusCode::CREATED,
            success_to_api_response(CreateMessageResponse {}),
        ),
        Err(e) => {
            let status = if e.to_string().contains("User is not a member") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, error_to_api_response(1, e.to_string()))
        }
    }
}

#[axum::debug_handler]
pub async fn get_messages(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<GetMessagesRequest>,
) -> impl IntoResponse {
    match MessageInfo::get_messages(&state.pool, req, &claims.sub).await {
        Ok(messages) => (StatusCode::OK, success_to_api_response(messages)),
        Err(e) => {
            let status = if e.to_string().contains("User is not a member") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, error_to_api_response(1, e.to_string()))
        }
    }
}
