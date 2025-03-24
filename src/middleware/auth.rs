use crate::{
    AppState,
    utils::{error_codes, error_to_api_response, verify_token},
};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tracing;

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
        None => return Err(unauthorized_response("缺少认证令牌")),
    };

    // 验证token
    let claims = match verify_token(token, config) {
        Ok(c) => c,
        Err(_) => return Err(unauthorized_response("无效或已过期的令牌")),
    };

    // 简化后的临时用户检查
    if claims.is_temp {
        let path = request.uri().path();
        tracing::info!("临时用户访问路径: {}", path);
        
        if path.starts_with("/api/users/update-password") || 
           path.starts_with("/api/users/reset-password") {
            tracing::info!("临时用户尝试访问受限功能，被拒绝");
            return Err(permission_denied_response("临时用户无法访问此功能"));
        }
        
        tracing::info!("临时用户访问被允许: {}", path);
    }

    // 注入用户ID到请求扩展
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

// 错误响应辅助函数
fn unauthorized_response(message: &str) -> Response {
    (
        StatusCode::OK,
        error_to_api_response::<()>(error_codes::AUTH_FAILED, message.to_string()),
    )
        .into_response()
}

// 权限不足的错误响应
fn permission_denied_response(message: &str) -> Response {
    (
        StatusCode::OK,
        error_to_api_response::<()>(error_codes::PERMISSION_DENIED, message.to_string()),
    )
        .into_response()
}
