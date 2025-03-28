use crate::utils::{error_codes, error_to_api_response};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing::error;

pub async fn log_errors(req: Request<Body>, next: Next) -> Response {
    tracing::info!("+++++++++++++++++, request: {:?}", req);
    let response = next.run(req).await;

    if response.status().is_server_error() {
        let (parts, body) = response.into_parts();
        let bytes = match to_bytes(body, 1024).await {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read error response body: {}", e);
                return (
                    StatusCode::OK,
                    error_to_api_response::<()>(
                        error_codes::INTERNAL_ERROR,
                        "服务器内部错误".to_string(),
                    ),
                )
                    .into_response();
            }
        };
        let body_str = String::from_utf8_lossy(&bytes);

        error!(
            "Server error occurred - Status: {}, Body: {}",
            parts.status, body_str
        );

        // 返回统一的API错误响应
        (
            StatusCode::OK,
            error_to_api_response::<()>(
                error_codes::INTERNAL_ERROR,
                format!("服务器内部错误: {}", body_str),
            ),
        )
            .into_response()
    } else {
        response
    }
}
