use crate::{AppState, utils::verify_token};
use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    // 直接使用state中的配置
    let config = &state.config;

    // 从请求头获取Authorization
    let headers = request.headers();
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t,
        None => return Err(unauthorized_response("Missing authentication token")),
    };

    // 验证token
    let claims = match verify_token(token, config) {
        Ok(c) => c,
        Err(_) => return Err(unauthorized_response("Invalid or expired token")),
    };

    // 简化后的临时用户检查
    if claims.is_temp {
        let path = request.uri().path();
        if path.starts_with("/api/users/update") || path.starts_with("/api/users/reset-password") {
            return Err(unauthorized_response(
                "Temporary users cannot access this feature",
            ));
        }
    }

    // 注入用户ID到请求扩展
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

// 错误响应辅助函数
fn unauthorized_response(message: &str) -> Response {
    (StatusCode::UNAUTHORIZED, Json(json!({ "error": message }))).into_response()
}
