use axum::{
    body::{Body, to_bytes},
    http::Request,
    middleware::Next,
    response::Response,
};
use tracing::error;

pub async fn log_errors(req: Request<Body>, next: Next) -> Response {
    let response = next.run(req).await;

    if response.status().is_server_error() {
        let (mut parts, body) = response.into_parts();
        let bytes = match to_bytes(body, 1024).await {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to read error response body: {}", e);
                return Response::from_parts(parts, axum::body::Body::empty());
            }
        };
        let body_str = String::from_utf8_lossy(&bytes);

        error!(
            "Server error occurred - Status: {}, Body: {}",
            parts.status, body_str
        );

        // 重置body以便重新构建响应
        parts.headers.remove(axum::http::header::CONTENT_LENGTH);
        Response::from_parts(parts, axum::body::Body::from(bytes))
    } else {
        response
    }
}
