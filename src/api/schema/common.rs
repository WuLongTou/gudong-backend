// 通用的数据结构定义

use serde::{Deserialize, Serialize};

/// 通用的API响应结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 错误码，0表示成功，非0表示失败
    pub code: i32,
    /// 错误消息，成功时为"success"
    pub msg: String,
    /// 响应数据，错误时为None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resp_data: Option<T>,
}

/// 空请求类型（用于无请求体的API）
#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyRequest {}

/// 空响应类型（用于无响应数据的API）
#[derive(Debug, Serialize, Deserialize)]
pub struct EmptyResponse {}

/// 分页信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    /// 当前页码
    pub page: u32,
    /// 每页数量
    pub page_size: u32,
    /// 总记录数
    pub total: u64,
}

/// 带分页的响应数据
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 分页信息
    pub pagination: Pagination,
}

/// 位置信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Location {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
} 