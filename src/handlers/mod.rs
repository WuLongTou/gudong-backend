use axum::Json;
use serde::Serialize;
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

mod group;
mod message;
mod user;

pub use group::{
    create_group, find_by_id, find_by_location, find_by_name, join_group, keep_alive, leave_group,
};
pub use message::{create_message, get_messages};
pub use user::{create_temporary, login, register, reset_password, update_user};

// 新增统一响应结构
#[derive(serde::Serialize)]
pub struct ApiResponse<T> {
    code: i32,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    resp_data: Option<T>,
}

// 修改所有 handler 返回类型为 Json<ApiResponse<T>>
pub fn success_to_api_response<T: Serialize>(data: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        code: 0,
        msg: "success".into(),
        resp_data: Some(data),
    })
}

pub fn error_to_api_response<T>(code: i32, msg: String) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        code,
        msg,
        resp_data: None,
    })
}
