// 群组实体
// 定义群组相关的数据库实体

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 群组实体，对应数据库中的群组表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupEntity {
    /// 群组ID
    pub id: String,
    /// 群组名称
    pub name: String,
    /// 创建者ID
    pub creator_id: String,
    /// 群组描述
    pub description: Option<String>,
    /// 群组密码（可选）
    pub password: Option<String>,
    /// 群组位置名称
    pub location_name: String,
    /// 群组位置纬度
    pub latitude: f64,
    /// 群组位置经度
    pub longitude: f64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active: DateTime<Utc>,
}

/// 群组成员实体，对应数据库中的群组成员表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMemberEntity {
    /// 记录ID
    pub id: String,
    /// 群组ID
    pub group_id: String,
    /// 用户ID
    pub user_id: String,
    /// 成员角色：0-普通成员，1-管理员，2-群主
    pub role: i32,
    /// 加入时间
    pub joined_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active: Option<DateTime<Utc>>,
}

/// 带有详细信息的群组结构
#[derive(Debug, sqlx::FromRow)]
pub struct GroupWithDetails {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub creator_id: String,
    pub creator_name: Option<String>,
    pub avatar_url: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub location_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_active_at: chrono::DateTime<chrono::Utc>,
    pub member_count: i64,
    pub creator_public_id: Option<String>,
}

/// 创建者基本信息
#[derive(Debug)]
pub struct CreatorInfo {
    pub nickname: String,
    pub public_user_id: String,
}
