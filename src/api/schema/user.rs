use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// 用户基本信息（响应）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub avatar_url: Option<String>,
}

// 创建注册用户请求
#[derive(Debug, Deserialize)]
pub struct CreateRegisteredUserRequest {
    pub user_id: String,
    pub password: String,
    pub nickname: String,
}

// 创建临时用户请求
#[derive(Debug, Deserialize)]
pub struct CreateTemporaryUserRequest {
    pub nickname: Option<String>,
}

// 创建用户响应
#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub user_id: String,
    pub nickname: String,
    pub token: String,
    pub expires_at: Option<i64>,
}

// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub user_id: String,
    pub password: String,
}

// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub token: String,
    pub nickname: String,
    pub expires_at: Option<i64>,
}

// 更新昵称请求
#[derive(Debug, Deserialize)]
pub struct UpdateNicknameRequest {
    pub nickname: String,
}

// 更新密码请求
#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

// 重置密码请求
#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub user_id: String,
    pub reset_code: String,
    pub new_password: String,
}

// 重置密码响应
#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
}

// 刷新令牌响应
#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    pub expires_at: Option<i64>,
}

// 检查令牌响应
#[derive(Debug, Serialize)]
pub struct CheckTokenResponse {
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
} 