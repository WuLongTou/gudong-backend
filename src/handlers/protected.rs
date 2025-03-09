pub async fn protected_handler(
    Extension(user_id): Extension<String>, // 从扩展获取用户ID
) -> impl IntoResponse {
    Json(json!({ "user_id": user_id }))
} 