use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 用户数据库实体
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserEntity {
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
    pub password_hash: Option<String>,
    pub recovery_code: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 用户公开ID，用于对外展示，保护用户真实ID
    pub public_user_id: String,
}

/// 用户权限数据库实体
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserPermissionEntity {
    pub user_id: String,
    pub permission: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 用户会话数据库实体
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct UserSessionEntity {
    pub session_id: String,
    pub user_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
