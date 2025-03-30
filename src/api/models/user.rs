use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 用户基本信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    /// 用户ID（公开ID，而非真实注册ID，保护用户安全）
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub avatar_url: Option<String>,
}

/// 注册新用户请求
#[derive(Debug, Deserialize)]
pub struct RegisterUserRequest {
    pub user_id: String,
    pub password: String,
    pub nickname: String,
}

/// 创建临时用户请求
#[derive(Debug, Deserialize)]
pub struct CreateTemporaryUserRequest {
    pub nickname: Option<String>,
}

/// 用户创建响应
#[derive(Debug, Serialize)]
pub struct UserCreationResponse {
    /// 用户ID（公开ID，而非真实注册ID，保护用户安全）
    pub user_id: String,
    pub nickname: String,
    pub token: String,
    pub expires_at: Option<i64>,
}

/// 用户登录请求
#[derive(Debug, Deserialize)]
pub struct UserLoginRequest {
    pub user_id: String,
    pub password: String,
}

/// 用户登录响应
#[derive(Debug, Serialize)]
pub struct UserLoginResponse {
    /// 用户ID（公开ID，而非真实注册ID，保护用户安全）
    pub user_id: String,
    pub token: String,
    pub nickname: String,
    pub expires_at: Option<i64>,
}

/// 更新用户昵称请求
#[derive(Debug, Deserialize)]
pub struct UpdateProfileNicknameRequest {
    pub nickname: String,
}

/// 更新用户密码请求
#[derive(Debug, Deserialize)]
pub struct UpdateProfilePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 重置密码请求
#[derive(Debug, Deserialize)]
pub struct ResetProfilePasswordRequest {
    pub user_id: String,
    pub reset_code: String,
    pub new_password: String,
}

/// 重置密码响应
#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub success: bool,
}

/// 刷新认证令牌响应
#[derive(Debug, Serialize)]
pub struct RefreshAuthTokenResponse {
    pub token: String,
    pub expires_at: Option<i64>,
}

/// 验证认证令牌响应
#[derive(Debug, Serialize)]
pub struct VerifyAuthTokenResponse {
    /// 用户ID（公开ID，而非真实注册ID，保护用户安全）
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
}
