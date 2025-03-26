use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    routes::activity::model::{NearbyUser, UserActivity},
    utils::{error_codes, error_to_api_response, success_to_api_response},
};

// 地理位置查询参数
#[derive(Debug, Deserialize)]
pub struct LocationQuery {
    latitude: Option<f64>,
    longitude: Option<f64>,
    radius: Option<f64>,
    limit: Option<i64>,
}

// 创建活动请求参数
#[derive(Debug, Deserialize)]
pub struct CreateActivityRequest {
    activity_type: String,
    activity_details: Option<String>,
    latitude: f64,
    longitude: f64,
}

// 创建活动响应
#[derive(Debug, Serialize)]
pub struct CreateActivityResponse {
    activity_id: String,
}

// 获取附近用户API
pub async fn find_nearby_users(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> Json<crate::utils::ApiResponse<Vec<NearbyUser>>> {
    // 检查必需的位置参数
    let latitude = match query.latitude {
        Some(lat) => lat,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少latitude参数".into()),
    };
    
    let longitude = match query.longitude {
        Some(lng) => lng,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少longitude参数".into()),
    };
    
    let radius = query.radius.unwrap_or(5000.0); // 默认5公里

    match NearbyUser::find_by_location(
        &state.pool,
        &state.redis,
        latitude,
        longitude,
        radius,
        query.limit,
    )
    .await
    {
        Ok(users) => success_to_api_response(users),
        Err(err) => {
            tracing::error!("查找附近用户错误: {:?}", err);
            error_to_api_response(error_codes::INTERNAL_ERROR, "获取附近用户失败".into())
        }
    }
}

// 获取最近活动API
pub async fn find_recent_activities(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> Json<crate::utils::ApiResponse<Vec<UserActivity>>> {
    // 检查必需的位置参数
    let latitude = match query.latitude {
        Some(lat) => lat,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少latitude参数".into()),
    };
    
    let longitude = match query.longitude {
        Some(lng) => lng,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少longitude参数".into()),
    };
    
    let radius = query.radius.unwrap_or(5000.0); // 默认5公里

    match UserActivity::find_recent_activities(
        &state.pool,
        &state.redis,
        latitude,
        longitude,
        radius,
        query.limit,
    )
    .await
    {
        Ok(activities) => success_to_api_response(activities),
        Err(err) => {
            tracing::error!("查找最近活动错误: {:?}", err);
            error_to_api_response(error_codes::INTERNAL_ERROR, "获取最近活动失败".into())
        }
    }
}

// 创建用户活动API
pub async fn create_user_activity(
    State(state): State<AppState>,
    axum::extract::Extension(user_id): axum::extract::Extension<String>,
    Json(request): Json<CreateActivityRequest>,
) -> Json<crate::utils::ApiResponse<CreateActivityResponse>> {
    match UserActivity::create(
        &state.pool,
        &state.redis,
        &user_id,
        &request.activity_type,
        request.activity_details.as_deref(),
        request.latitude,
        request.longitude,
    )
    .await
    {
        Ok(activity_id) => success_to_api_response(CreateActivityResponse { activity_id }),
        Err(err) => {
            tracing::error!("创建用户活动错误: {:?}", err);
            error_to_api_response(error_codes::INTERNAL_ERROR, "创建活动失败".into())
        }
    }
}

// 获取附近活动
pub async fn find_nearby_activities(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> Json<crate::utils::ApiResponse<Vec<UserActivity>>> {
    // 检查必需的位置参数
    let latitude = match query.latitude {
        Some(lat) => lat,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少latitude参数".into()),
    };
    
    let longitude = match query.longitude {
        Some(lng) => lng,
        None => return error_to_api_response(error_codes::VALIDATION_ERROR, "缺少longitude参数".into()),
    };
    
    let radius = query.radius.unwrap_or(5000.0); // 默认5公里

    match UserActivity::find_nearby_activities(
        &state.pool,
        &state.redis,
        latitude,
        longitude,
        radius,
        query.limit,
    )
    .await
    {
        Ok(activities) => success_to_api_response(activities),
        Err(err) => {
            tracing::error!("查找附近活动错误: {:?}", err);
            error_to_api_response(error_codes::INTERNAL_ERROR, "获取附近活动失败".into())
        }
    }
}

// 获取用户活动历史
pub async fn find_user_activities(
    State(state): State<AppState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
    axum::extract::Extension(current_user_id): axum::extract::Extension<String>,
    Query(query): Query<LocationQuery>,
) -> Json<crate::utils::ApiResponse<Vec<UserActivity>>> {
    // 如果是当前用户的活动，从认证中间件获取用户ID
    let actual_user_id = if user_id == "me" {
        current_user_id
    } else {
        user_id
    };
    
    match UserActivity::find_by_user_id(
        &state.pool,
        &actual_user_id,
        query.limit,
    )
    .await
    {
        Ok(activities) => success_to_api_response(activities),
        Err(err) => {
            tracing::error!("查找用户活动历史错误: {:?}", err);
            error_to_api_response(error_codes::INTERNAL_ERROR, "获取用户活动历史失败".into())
        }
    }
}
