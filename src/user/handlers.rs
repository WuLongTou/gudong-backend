use crate::result::ApiResult;
use crate::user::types::{RegisterUserRequest, RegisterUserResponse};
use axum::Json;
use serde_json::json;

pub(crate) async fn register_user(
    Json(req): Json<RegisterUserRequest>,
) -> Json<ApiResult<RegisterUserResponse>> {
    tracing::info!("register_user: {:?}", serde_json::to_string(&req).unwrap());
    Json(ApiResult::success(RegisterUserResponse {
        session_token: "123".to_string(),
        server_public_key: json!({"alg": "RSA-OAEP"}),
        expires_at: 123,
    }))
}
