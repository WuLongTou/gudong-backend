use serde::{Deserialize, Serialize};

/// 速率限制缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedRateLimit {
    pub key: String,
    pub count: u32,
    pub reset_at: i64, // Unix timestamp
} 