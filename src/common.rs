use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(crate) struct ApiResult<T> {
    pub code: i32,
    pub message: Option<String>,
    pub data: T,
}

// 公共数据结构
#[derive(Deserialize, Serialize, Clone)]
pub(crate) struct MapLocation {
    pub latitude: f64,
    pub longitude: f64,
}
