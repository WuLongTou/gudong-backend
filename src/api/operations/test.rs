use crate::utils::success_to_api_response;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;

/// Ping响应
#[derive(Serialize)]
pub struct PingResponse {
    /// 服务状态
    pub status: String,
    /// 服务器时间
    pub timestamp: i64,
}

/// 健康检查接口
pub async fn ping() -> impl IntoResponse {
    let now = chrono::Utc::now();

    (
        StatusCode::OK,
        success_to_api_response(PingResponse {
            status: "ok".to_string(),
            timestamp: now.timestamp(),
        }),
    )
}
