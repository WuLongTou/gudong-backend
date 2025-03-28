use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 附近用户数据模型(用于缓存和API交互)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NearbyUser {
    pub user_id: String,
    pub nickname: String,
    pub last_active: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub distance: f64,
    pub avatar: Option<String>,
    pub status: String,
}

/// 用户活动数据模型(用于缓存和API交互)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserActivity {
    pub activity_id: String,
    pub user_id: String,
    pub nickname: String,
    pub activity_type: String,
    pub activity_details: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: DateTime<Utc>,
    pub distance: f64,
    pub avatar: Option<String>,
}

/// 附近用户缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedNearbyUser {
    pub user_id: String,
    pub nickname: String,
    pub last_active: i64, // Unix timestamp
    pub latitude: f64,
    pub longitude: f64,
    pub distance: f64,
    pub avatar: Option<String>,
    pub status: String,
}

/// 用户活动缓存数据模型
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedUserActivity {
    pub activity_id: String,
    pub user_id: String,
    pub nickname: String,
    pub activity_type: String,
    pub activity_details: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub created_at: i64, // Unix timestamp
    pub distance: f64,
    pub avatar: Option<String>,
}

impl From<&NearbyUser> for CachedNearbyUser {
    fn from(user: &NearbyUser) -> Self {
        Self {
            user_id: user.user_id.clone(),
            nickname: user.nickname.clone(),
            last_active: user.last_active.timestamp(),
            latitude: user.latitude,
            longitude: user.longitude,
            distance: user.distance,
            avatar: user.avatar.clone(),
            status: user.status.clone(),
        }
    }
}

impl From<CachedNearbyUser> for NearbyUser {
    fn from(cached: CachedNearbyUser) -> Self {
        Self {
            user_id: cached.user_id,
            nickname: cached.nickname,
            last_active: DateTime::from_timestamp(cached.last_active, 0).unwrap_or_else(|| Utc::now()),
            latitude: cached.latitude,
            longitude: cached.longitude,
            distance: cached.distance,
            avatar: cached.avatar,
            status: cached.status,
        }
    }
}

impl From<&UserActivity> for CachedUserActivity {
    fn from(activity: &UserActivity) -> Self {
        Self {
            activity_id: activity.activity_id.clone(),
            user_id: activity.user_id.clone(),
            nickname: activity.nickname.clone(),
            activity_type: activity.activity_type.clone(),
            activity_details: activity.activity_details.clone(),
            latitude: activity.latitude,
            longitude: activity.longitude,
            created_at: activity.created_at.timestamp(),
            distance: activity.distance,
            avatar: activity.avatar.clone(),
        }
    }
}

impl From<CachedUserActivity> for UserActivity {
    fn from(cached: CachedUserActivity) -> Self {
        Self {
            activity_id: cached.activity_id,
            user_id: cached.user_id,
            nickname: cached.nickname,
            activity_type: cached.activity_type,
            activity_details: cached.activity_details,
            latitude: cached.latitude,
            longitude: cached.longitude,
            created_at: DateTime::from_timestamp(cached.created_at, 0).unwrap_or_else(|| Utc::now()),
            distance: cached.distance,
            avatar: cached.avatar,
        }
    }
} 