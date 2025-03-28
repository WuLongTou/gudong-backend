use serde::{Deserialize, Serialize};

/// 用户缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedUser {
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
    pub created_at: i64, // Unix timestamp
}

/// 用户在线状态缓存
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedUserStatus {
    pub user_id: String,
    pub online: bool,
    pub last_activity: i64, // Unix timestamp
    pub location: Option<CachedUserLocation>,
}

/// 用户位置缓存
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedUserLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub update_time: i64, // Unix timestamp
} 