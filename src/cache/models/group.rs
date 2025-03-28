use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 群组数据模型(用于缓存和API交互)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub group_id: String,
    pub name: String,
    pub location_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
    pub password_hash: Option<String>,
    pub creator_id: String,
    pub created_at: DateTime<Utc>,
    pub member_count: i32,
}

/// 群组缓存模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedGroup {
    pub group_id: String,
    pub name: String,
    pub location_name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub description: Option<String>,
    pub password_hash: Option<String>,
    pub creator_id: String,
    pub created_at: i64, // Unix timestamp
    pub member_count: i32,
}

/// 附近群组缓存模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedNearbyGroup {
    pub group_id: String,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub distance: f64,
    pub member_count: i32,
}

/// 群组成员缓存模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedGroupMember {
    pub user_id: String,
    pub nickname: String,
    pub join_time: i64, // Unix timestamp
    pub is_admin: bool,
    pub avatar: Option<String>,
}

impl From<&Group> for CachedGroup {
    fn from(group: &Group) -> Self {
        Self {
            group_id: group.group_id.clone(),
            name: group.name.clone(),
            location_name: group.location_name.clone(),
            latitude: group.latitude,
            longitude: group.longitude,
            description: group.description.clone(),
            password_hash: group.password_hash.clone(),
            creator_id: group.creator_id.clone(),
            created_at: group.created_at.timestamp(),
            member_count: group.member_count,
        }
    }
}

impl From<CachedGroup> for Group {
    fn from(cached: CachedGroup) -> Self {
        Self {
            group_id: cached.group_id,
            name: cached.name,
            location_name: cached.location_name,
            latitude: cached.latitude,
            longitude: cached.longitude,
            description: cached.description,
            password_hash: cached.password_hash,
            creator_id: cached.creator_id,
            created_at: DateTime::from_timestamp(cached.created_at, 0).unwrap_or_else(|| Utc::now()),
            member_count: cached.member_count,
        }
    }
} 