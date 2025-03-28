// 群组相关的数据结构定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 创建群组的请求
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

/// 创建群组的响应
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateGroupResponse {
    /// 新创建的群组ID
    pub group_id: String,
}

/// 按名称查询群组的请求
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryGroupsByNameRequest {
    /// 群组名称（支持模糊匹配）
    pub name: String,
}

/// 按位置查询群组的请求
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryGroupsByLocationRequest {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 搜索半径（米）
    pub radius: u32,
}

/// 加入群组的请求
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinGroupRequest {
    /// 群组ID
    pub group_id: String,
    /// 可选的密码（如果群组需要密码）
    pub password: Option<String>,
}

/// 加入群组的响应
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinGroupResponse {
    /// 加入成功的标志
    pub success: bool,
}

/// 附近群组信息
#[derive(Debug, Serialize, Deserialize)]
pub struct NearbyGroup {
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

/// 群组成员信息
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMember {
    /// 用户ID
    pub user_id: String,
    /// 用户昵称
    pub nickname: String,
    /// 最后活跃时间
    pub last_active: DateTime<Utc>,
}

/// 设置群组成员角色的请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SetMemberRoleRequest {
    /// 是否设置为管理员
    pub is_admin: bool,
} 