// 活动实体
// 定义活动相关的数据库实体

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 活动类型枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i32)]
pub enum ActivityType {
    /// 用户登录
    UserLogin = 1,
    /// 用户签到
    UserCheckIn = 2,
    /// 创建群组
    GroupCreated = 10,
    /// 加入群组
    UserJoined = 11,
    /// 离开群组
    UserLeft = 12,
    /// 发送消息
    MessageSent = 20,
}

/// 活动实体，对应数据库中的活动表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityEntity {
    /// 活动ID
    pub id: String,
    /// 活动类型
    #[sqlx(rename = "activity_type")]
    pub activity_type: i32,
    /// 用户ID
    pub user_id: String,
    /// 相关群组ID（可选）
    pub group_id: Option<String>,
    /// 活动内容（扩展数据，通常为JSON格式）
    pub content: Option<String>,
    /// 活动描述
    pub description: String,
    /// 活动发生时的经度
    pub longitude: f64,
    /// 活动发生时的纬度
    pub latitude: f64,
    /// 活动发生时间
    pub created_at: DateTime<Utc>,
}

impl ActivityEntity {
    /// 获取活动类型
    pub fn get_activity_type(&self) -> ActivityType {
        match self.activity_type {
            1 => ActivityType::UserLogin,
            2 => ActivityType::UserCheckIn,
            10 => ActivityType::GroupCreated,
            11 => ActivityType::UserJoined,
            12 => ActivityType::UserLeft,
            20 => ActivityType::MessageSent,
            _ => ActivityType::UserCheckIn, // 默认为签到
        }
    }

    /// 设置活动类型
    pub fn set_activity_type(&mut self, activity_type: ActivityType) {
        self.activity_type = activity_type as i32;
    }
}

/// 近期用户活动信息
pub struct NearbyUserActivity {
    pub user_id: String,
    pub nickname: String,
    pub last_activity_id: Option<String>,
    pub last_activity_type: Option<String>,
    pub last_activity_description: Option<String>,
    pub last_activity_time: Option<chrono::DateTime<chrono::Utc>>,
    pub distance: Option<f64>,
}
