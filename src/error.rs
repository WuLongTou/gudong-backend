use axum::Json;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
#[derive(Debug)]
pub enum AppError {
    Unauthorized = 1,
    InternalServerError,
    FailedToGetMessage,
    FailedToStoreMessage,
    FailedToStoreGroup,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: i32,
    error_message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "未授权访问".into()),
            AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "内部服务器错误".to_string(),
            ),
            AppError::FailedToGetMessage => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "获取消息失败".to_string(),
            ),
            AppError::FailedToStoreMessage => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "存储消息失败".to_string(),
            ),
            AppError::FailedToStoreGroup => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "存储群组失败".to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            code: status.as_u16() as i32,
            error_message,
        });

        (status, body).into_response()
    }
}
