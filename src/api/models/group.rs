// 群组相关的数据结构定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ------------------------
// API 请求参数类型
// ------------------------

/// 创建群组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroupRequest {
    /// 群组名称
    pub name: String,
    /// 群组位置名称（如城市、地区等）
    pub location_name: String,
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 群组描述
    pub description: Option<String>,
    /// 可选的群组密码
    pub password: Option<String>,
}

/// 搜索附近群组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchNearbyGroupsRequest {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 搜索半径（米），默认5000米
    #[serde(default = "default_radius")]
    pub radius: u32,
}

fn default_radius() -> u32 {
    5000
}

/// 加入群组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinGroupRequest {
    /// 可选的密码（如果群组需要密码）
    pub password: Option<String>,
}

/// 更新群组成员角色请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateMemberRoleRequest {
    /// 角色名称
    pub role: String,
}

// ------------------------
// API 响应数据类型
// ------------------------

/// 群组创建响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupCreationResponse {
    /// 新创建的群组ID
    pub group_id: String,
}

/// 群组成员信息
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMemberProfile {
    /// 用户公开ID
    pub public_user_id: String,
    /// 用户昵称
    pub nickname: String,
    /// 最后活跃时间
    pub last_active: DateTime<Utc>,
    /// 用户角色
    pub role: String,
}

/// 加入群组响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupJoinResponse {
    /// 加入成功的标志
    pub success: bool,
}

/// 群组详细信息
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupDetailedInfo {
    /// 群组ID
    pub group_id: String,
    /// 群组名称
    pub name: String,
    /// 群组描述
    pub description: Option<String>,
    /// 创建者公开ID（对外展示的ID）
    pub public_creator_id: String,
    /// 创建者昵称
    pub creator_name: Option<String>,
    /// 创建者头像URL
    pub avatar_url: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 成员数量
    pub member_count: i64,
    /// 距离（米）
    pub distance: f64,
    /// 位置名称
    pub location_name: String,
    /// 是否需要密码才能加入
    pub is_password_required: bool,
}

/// 群组心跳请求
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupHeartbeatRequest {
    /// 群组ID
    pub group_id: String,
}

/// 附近群组信息
#[derive(Debug, Serialize, Deserialize)]
pub struct NearbyGroupInfo {
    /// 群组ID
    pub id: String,
    /// 群组名称
    pub name: String,
    /// 群组描述
    pub description: Option<String>,
    /// 成员数量
    pub member_count: u32,
    /// 距离（米）
    pub distance: Option<f64>,
}

/// 带密码的加入群组请求
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinGroupWithPasswordRequest {
    /// 密码
    pub password: Option<String>,
}

/// 设置成员角色请求 (别名，与UpdateMemberRoleRequest相同)
pub type SetMemberRoleRequest = UpdateMemberRoleRequest;

/// 创建群组响应
pub type CreateGroupResponse = GroupCreationResponse;

/// 群组详情 (别名，与GroupDetailedInfo相同)
pub type GroupDetail = GroupDetailedInfo;

/// 群组成员 (别名，与GroupMemberProfile相同)
pub type GroupMember = GroupMemberProfile;

/// 加入群组响应 (别名，与GroupJoinResponse相同)
pub type JoinGroupResponse = GroupJoinResponse;

/// 附近群组请求 (别名，与SearchNearbyGroupsRequest相同)
pub type NearbyGroupsRequest = SearchNearbyGroupsRequest; 