use axum::{middleware::Next, http::Request};

async fn auth_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Result<axum::response::Response, AppError> {
    let auth_header = request.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(auth_header) if is_valid_token(auth_header) => {
            Ok(next.run(request).await)
        }
        _ => Err(AppError::Unauthorized),
    }
}

async fn verify_token(token: Option<&str>) -> Result<String, Response> {
    // 实现token验证和用户ID解析逻辑
} 