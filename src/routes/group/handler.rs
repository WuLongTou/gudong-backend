use axum::{
    Extension,
    extract::{Json, Query, State, Path},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;

use super::model::{CreateGroupRequest, Group, GroupInfo, JoinGroupRequest, KeepAliveRequest};
use crate::routes::activity::model::{UserActivity, activity_types};
use crate::utils::{Claims, error_codes, error_to_api_response, success_to_api_response};

#[derive(Debug, Deserialize)]
pub struct LocationQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub radius: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct NameQuery {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub group_id: String,
}

#[derive(Debug, Serialize)]
pub struct KeepAliveResponse {
    pub last_active_time: DateTime<Utc>,
}

#[axum::debug_handler]
pub async fn create_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    match Group::create(&state.pool, req.clone(), claims.sub.clone()).await {
        Ok(group) => {
            // 创建群组活动记录
            let activity_details = format!("创建了群组「{}」", group.name);
            let _ = UserActivity::create(
                &state.pool,
                &state.redis,
                &claims.sub,
                activity_types::GROUP_CREATE,
                Some(&activity_details),
                group.latitude,
                group.longitude,
            )
            .await;
            
            (
                StatusCode::CREATED,
                success_to_api_response(GroupInfo::from(group)),
            )
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_id(
    State(state): State<AppState>,
    Query(query): Query<IdQuery>,
) -> impl IntoResponse {
    match Group::find_by_id(&state.pool, &state.redis, &query.group_id).await {
        Ok(Some(group)) => (
            StatusCode::OK,
            success_to_api_response(GroupInfo::from(group)),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            error_to_api_response(error_codes::NOT_FOUND, "Group not found".to_string()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_name(
    State(state): State<AppState>,
    Query(query): Query<NameQuery>,
) -> impl IntoResponse {
    match Group::find_by_name(&state.pool, &state.redis, &query.name).await {
        Ok(groups) => {
            let group_infos = groups.into_iter().map(GroupInfo::from).collect::<Vec<_>>();
            (StatusCode::OK, success_to_api_response(group_infos))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_location(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> impl IntoResponse {
    let radius = query
        .radius
        .unwrap_or(1000.0)
        .min(state.config.max_search_radius);

    match Group::find_by_location(
        &state.pool,
        &state.redis,
        query.latitude,
        query.longitude,
        radius,
    )
    .await
    {
        Ok(groups) => {
            let group_infos = groups.into_iter().map(GroupInfo::from).collect::<Vec<_>>();
            (StatusCode::OK, success_to_api_response(group_infos))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn join_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<JoinGroupRequest>,
) -> impl IntoResponse {
    match Group::join(
        &state.pool,
        &state.redis,
        &req.group_id,
        &claims.sub,
        req.password,
    )
    .await
    {
        Ok(group) => {
            // 创建加入群组活动记录
            let activity_details = format!("加入了群组「{}」", group.name);
            let _ = UserActivity::create(
                &state.pool,
                &state.redis,
                &claims.sub,
                activity_types::GROUP_JOIN,
                Some(&activity_details),
                group.latitude,
                group.longitude,
            )
            .await;
            
            (
                StatusCode::OK,
                success_to_api_response(serde_json::json!({
                    "success": true
                })),
            )
        },
        Err(e) => {
            let status = if e.to_string().contains("Password required")
                || e.to_string().contains("Invalid password")
            {
                StatusCode::FORBIDDEN
            } else if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn leave_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<IdQuery>,
) -> impl IntoResponse {
    // 首先获取群组信息，以便用于创建活动
    let group_result = Group::find_by_id(&state.pool, &state.redis, &req.group_id).await;
    
    match Group::leave(&state.pool, &state.redis, &req.group_id, &claims.sub).await {
        Ok(_) => {
            // 如果我们能获取群组信息，创建离开群组活动记录
            if let Ok(Some(group)) = group_result {
                let activity_details = format!("离开了群组「{}」", group.name);
                let _ = UserActivity::create(
                    &state.pool,
                    &state.redis,
                    &claims.sub,
                    activity_types::GROUP_LEAVE,
                    Some(&activity_details),
                    group.latitude,
                    group.longitude,
                )
                .await;
            }
            
            (
                StatusCode::OK,
                success_to_api_response(serde_json::json!({
                    "success": true
                })),
            )
        },
        Err(e) => {
            let status = if e.to_string().contains("User not in group") {
                StatusCode::BAD_REQUEST
            } else if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn keep_alive(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<KeepAliveRequest>,
) -> impl IntoResponse {
    match Group::keep_alive(&state.pool, &req.group_id, &claims.sub).await {
        Ok(last_active) => (
            StatusCode::OK,
            success_to_api_response(serde_json::json!({
                "last_active": last_active
            })),
        ),
        Err(e) => {
            let status = if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

// 获取群组详情
pub async fn get_group_detail(
    State(state): State<AppState>,
    axum::extract::Path(group_id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    // 实际实现会根据群组ID查询详情
    // 这里暂时返回一个空响应
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": {
            "id": group_id,
            "name": "",
            "description": "",
            "memberCount": 0,
            "createdAt": ""
        }
    }))
}

// 获取用户所在的群组
pub async fn get_user_groups(
    // 参数和实现
) -> impl axum::response::IntoResponse {
    // 暂时返回一个空响应，后续实现
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": {
            "items": [],
            "total": 0
        }
    }))
}

// 获取附近的群组
pub async fn find_nearby_groups(
    // 参数和实现
) -> impl axum::response::IntoResponse {
    // 暂时返回一个空响应，后续实现
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": {
            "items": [],
            "total": 0,
            "page": 1,
            "pageSize": 10,
            "totalPages": 0
        }
    }))
}

// 获取群组成员
pub async fn get_group_members(
    State(state): State<AppState>,
    axum::extract::Path(group_id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    // 实际实现会根据群组ID查询成员
    // 这里暂时返回一个空响应
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": {
            "items": [],
            "total": 0
        }
    }))
}

// 移除群组成员
pub async fn remove_group_member(
    State(state): State<AppState>,
    axum::extract::Path((group_id, user_id)): axum::extract::Path<(String, String)>,
    Extension(claims): Extension<Claims>,
) -> impl axum::response::IntoResponse {
    // 实际实现会验证当前用户权限并移除成员
    // 这里暂时返回一个空响应
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": null
    }))
}

// 设置成员角色
pub async fn set_member_role(
    State(state): State<AppState>,
    axum::extract::Path((group_id, user_id)): axum::extract::Path<(String, String)>,
    Extension(claims): Extension<Claims>,
    Json(role_data): Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    // 实际实现会验证当前用户权限并设置角色
    // 这里暂时返回一个空响应
    axum::Json(serde_json::json!({
        "code": 0,
        "msg": "success",
        "resp_data": null
    }))
}
