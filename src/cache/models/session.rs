use serde::{Deserialize, Serialize};

/// 会话缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedSession {
    pub session_id: String,
    pub user_id: String,
    pub data: Option<String>,
    pub created_at: i64, // Unix timestamp
    pub expires_at: i64, // Unix timestamp
}
