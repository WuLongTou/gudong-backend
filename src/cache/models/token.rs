use serde::{Deserialize, Serialize};

/// 令牌缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: i64, // Unix timestamp
} 