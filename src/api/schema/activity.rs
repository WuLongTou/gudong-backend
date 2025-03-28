// 活动相关的数据结构定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 活动类型枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityType {
    /// 群组创建
    GroupCreated,
    /// 用户加入群组
    UserJoined,
    /// 消息发送
    MessageSent,
    /// 用户签到
    UserCheckedIn,
}

/// 获取附近活动请求
#[derive(Debug, Serialize, Deserialize)]
pub struct GetNearbyActivitiesRequest {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 搜索半径（米）
    pub radius: u32,
    /// 活动数量限制
    pub limit: u32,
}

/// 活动详情
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityDetail {
    /// 活动ID
    pub id: String,
    /// 活动类型
    pub activity_type: ActivityType,
    /// 关联的群组ID
    pub group_id: String,
    /// 群组名称
    pub group_name: String,
    /// 关联的用户ID
    pub user_id: String,
    /// 用户名称
    pub user_name: String,
    /// 活动描述
    pub description: String,
    /// 发生时间
    pub occurred_at: DateTime<Utc>,
    /// 发生位置的纬度
    pub latitude: f64,
    /// 发生位置的经度
    pub longitude: f64,
    /// 与查询位置的距离（米）
    pub distance: Option<f64>,
}

/// 获取附近活动响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GetNearbyActivitiesResponse {
    /// 活动列表
    pub activities: Vec<ActivityDetail>,
}

/// 创建用户活动请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserActivityRequest {
    /// 活动类型
    pub activity_type: ActivityType,
    /// 活动描述
    pub description: Option<String>,
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
}

/// 创建用户活动响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserActivityResponse {
    /// 活动ID
    pub activity_id: String,
}

/// 查找附近用户请求
#[derive(Debug, Serialize, Deserialize)]
pub struct FindNearbyUsersRequest {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 搜索半径（米）
    pub radius: u32,
    /// 用户数量限制
    pub limit: u32,
}

/// 附近用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct NearbyUser {
    /// 用户ID
    pub user_id: String,
    /// 用户昵称
    pub nickname: String,
    /// 距离（米）
    pub distance: f64,
}

/// 查找附近用户响应
#[derive(Debug, Serialize, Deserialize)]
pub struct FindNearbyUsersResponse {
    /// 用户列表
    pub users: Vec<NearbyUser>,
}

/// 查找用户活动请求
#[derive(Debug, Serialize, Deserialize)]
pub struct FindUserActivitiesRequest {
    /// 活动数量限制
    pub limit: u32,
}

/// 查找用户活动响应
#[derive(Debug, Serialize, Deserialize)]
pub struct FindUserActivitiesResponse {
    /// 活动列表
    pub activities: Vec<ActivityDetail>,
    /// 下一页游标
    pub next_cursor: Option<String>,
    /// 是否还有更多
    pub has_more: bool,
}

/// 查找群组活动请求
#[derive(Debug, Serialize, Deserialize)]
pub struct FindGroupActivitiesRequest {
    /// 活动数量限制
    pub limit: u32,
}

/// 查找群组活动响应
#[derive(Debug, Serialize, Deserialize)]
pub struct FindGroupActivitiesResponse {
    /// 活动列表
    pub activities: Vec<ActivityDetail>,
}

/// 获取全部活动请求
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAllActivitiesRequest {
    /// 活动数量限制
    pub limit: Option<u32>,
}

/// 获取全部活动响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GetAllActivitiesResponse {
    /// 活动列表
    pub activities: Vec<ActivityDetail>,
} 