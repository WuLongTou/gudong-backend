// 活动处理器
// 处理活动相关的API请求

use crate::AppState;
use crate::api::models::activity::*;
use crate::database::operations::activity::ActivityOperation;
use crate::utils::Claims;
use crate::utils::{error_codes, error_to_api_response, success_to_api_response};
use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

/// 获取附近活动
pub async fn get_nearby_activities(
    State(state): State<AppState>,
    Query(params): Query<GetNearbyActivitiesRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        "请求获取半径 {}m 内的附近活动，坐标: ({}, {})",
        params.radius,
        params.latitude,
        params.longitude
    );

    // 将搜索半径限制在配置的最大值内
    let radius = params.radius as f64;
    let radius = radius.min(state.config.max_search_radius);

    // 验证纬度和经度范围，纬度[-90,90]，经度[-180,180]
    if params.latitude < -90.0
        || params.latitude > 90.0
        || params.longitude < -180.0
        || params.longitude > 180.0
    {
        tracing::warn!(
            "非法的地理坐标: 经度={}, 纬度={}",
            params.longitude,
            params.latitude
        );
        return (
            StatusCode::OK,
            error_to_api_response::<GetNearbyActivitiesResponse>(
                error_codes::VALIDATION_ERROR,
                "非法的地理坐标".to_string(),
            ),
        );
    }

    // 创建活动存储库实例
    let repo = Arc::new(ActivityOperation::new(Arc::new(state.pool.clone())));

    // 从数据库获取附近活动，只获取用户签到类型的活动
    match repo
        .find_nearby_activities_by_type(
            params.latitude,
            params.longitude,
            radius,
            params.limit as i64,
            &["USER_CHECKIN"],
        )
        .await
    {
        Ok(activities) => {
            tracing::debug!("从数据库获取到 {} 条附近活动", activities.len());

            // 将数据库实体转换为API响应格式
            let activity_details: Vec<ActivityDetail> = activities
                .into_iter()
                .map(|activity| ActivityDetail {
                    id: activity.id,
                    activity_type: match activity.activity_type {
                        2 => ActivityType::UserCheckedIn,
                        10 => ActivityType::GroupCreated,
                        11 => ActivityType::UserJoined,
                        20 => ActivityType::MessageSent,
                        _ => ActivityType::UserCheckedIn,
                    },
                    group_id: activity.group_id.unwrap_or_default(),
                    group_name: String::new(),
                    user_id: activity.user_id,
                    user_name: String::new(),
                    description: activity.description,
                    occurred_at: activity.created_at,
                    latitude: activity.latitude,
                    longitude: activity.longitude,
                    distance: None,
                })
                .collect();

            // 限制返回数量
            let mut result = activity_details;
            let limit = params.limit as usize;
            if result.len() > limit {
                result.truncate(limit);
            }

            (
                StatusCode::OK,
                success_to_api_response(GetNearbyActivitiesResponse { activities: result }),
            )
        }
        Err(e) => {
            tracing::error!("获取附近活动失败: {}", e);
            (
                StatusCode::OK,
                error_to_api_response::<GetNearbyActivitiesResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取附近活动失败: {}", e),
                ),
            )
        }
    }
}

/// 创建用户活动
pub async fn create_user_activity(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateUserActivityRequest>,
) -> impl IntoResponse {
    let user_id = &claims.sub;
    tracing::debug!("用户 {} 尝试创建活动", user_id);

    // 验证纬度和经度范围，纬度[-90,90]，经度[-180,180]
    if payload.latitude < -90.0
        || payload.latitude > 90.0
        || payload.longitude < -180.0
        || payload.longitude > 180.0
    {
        tracing::warn!(
            "用户 {} 提供了非法的地理坐标: 经度={}, 纬度={}",
            user_id,
            payload.longitude,
            payload.latitude
        );
        return (
            StatusCode::OK,
            error_to_api_response::<CreateUserActivityResponse>(
                error_codes::VALIDATION_ERROR,
                "非法的地理坐标".to_string(),
            ),
        );
    }

    // 创建活动存储库实例
    let repo = Arc::new(ActivityOperation::new(Arc::new(state.pool.clone())));

    // 将枚举转换为字符串
    let activity_type_str = match payload.activity_type {
        ActivityType::UserCheckedIn => "USER_CHECKIN",
        ActivityType::GroupCreated => "GROUP_CREATE",
        ActivityType::UserJoined => "USER_JOINED",
        ActivityType::MessageSent => "MESSAGE_SENT",
    };

    // 创建活动
    match repo
        .create_activity(
            user_id,
            activity_type_str,
            payload.description.as_deref(),
            payload.latitude,
            payload.longitude,
        )
        .await
    {
        Ok(activity_id) => {
            tracing::info!("用户 {} 成功创建活动 {}", user_id, activity_id);

            // 返回活动ID
            (
                StatusCode::OK,
                success_to_api_response(CreateUserActivityResponse { activity_id }),
            )
        }
        Err(e) => {
            tracing::error!("用户 {} 创建活动失败: {}", user_id, e);
            (
                StatusCode::OK,
                error_to_api_response::<CreateUserActivityResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("创建活动失败: {}", e),
                ),
            )
        }
    }
}

/// 获取用户活动
pub async fn find_user_activities(
    State(state): State<AppState>,
    path: Option<Path<String>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<FindUserActivitiesRequest>,
) -> impl IntoResponse {
    // 确定要查询的用户ID：如果路径参数存在，使用路径参数；否则使用当前认证用户
    let target_user_id = if let Some(Path(user_id)) = path {
        user_id
    } else {
        claims.sub.clone()
    };

    tracing::debug!(
        "用户 {} 请求查看用户 {} 的活动记录",
        claims.sub,
        target_user_id
    );

    // 创建活动存储库实例
    let repo = Arc::new(ActivityOperation::new(Arc::new(state.pool.clone())));

    // 获取用户活动列表
    match repo
        .find_user_activities(&target_user_id, params.limit as i64)
        .await
    {
        Ok(activities) => {
            tracing::debug!(
                "成功获取用户 {} 的 {} 条活动记录",
                target_user_id,
                activities.len()
            );

            // 将数据库实体转换为API响应格式
            let activity_details = activities
                .into_iter()
                .map(|activity| ActivityDetail {
                    id: activity.id,
                    activity_type: match activity.activity_type {
                        2 => ActivityType::UserCheckedIn,
                        10 => ActivityType::GroupCreated,
                        11 => ActivityType::UserJoined,
                        20 => ActivityType::MessageSent,
                        _ => ActivityType::UserCheckedIn,
                    },
                    group_id: activity.group_id.unwrap_or_default(),
                    group_name: String::new(),
                    user_id: activity.user_id,
                    user_name: String::new(),
                    description: activity.description,
                    occurred_at: activity.created_at,
                    latitude: activity.latitude,
                    longitude: activity.longitude,
                    distance: None,
                })
                .collect::<Vec<_>>();

            // 计算下一页游标
            let next_cursor = if !activity_details.is_empty() {
                Some(activity_details.last().unwrap().id.clone())
            } else {
                None
            };

            // 判断是否还有更多记录
            let has_more = activity_details.len() == params.limit as usize;

            (
                StatusCode::OK,
                success_to_api_response(FindUserActivitiesResponse {
                    activities: activity_details,
                    next_cursor,
                    has_more,
                }),
            )
        }
        Err(e) => {
            tracing::error!("获取用户 {} 的活动记录失败: {}", target_user_id, e);
            (
                StatusCode::OK,
                error_to_api_response::<FindUserActivitiesResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取用户活动记录失败: {}", e),
                ),
            )
        }
    }
}

/// 获取群组活动
pub async fn get_group_activities(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
    Query(params): Query<FindGroupActivitiesRequest>,
) -> impl IntoResponse {
    // 创建活动仓库实例
    let activity_repo = ActivityOperation::new(Arc::new(state.pool.clone()));

    match activity_repo
        .find_group_activities(&group_id, params.limit as i64)
        .await
    {
        Ok(activities) => {
            // 转换为API响应格式
            let activity_details = activities
                .into_iter()
                .map(|activity| ActivityDetail {
                    id: activity.id,
                    activity_type: match activity.activity_type {
                        2 => ActivityType::UserCheckedIn,
                        10 => ActivityType::GroupCreated,
                        11 => ActivityType::UserJoined,
                        20 => ActivityType::MessageSent,
                        _ => ActivityType::UserCheckedIn,
                    },
                    group_id: activity.group_id.unwrap_or_default(),
                    group_name: String::new(),
                    user_id: activity.user_id,
                    user_name: String::new(),
                    description: activity.description,
                    occurred_at: activity.created_at,
                    latitude: activity.latitude,
                    longitude: activity.longitude,
                    distance: None,
                })
                .collect();

            (
                StatusCode::OK,
                success_to_api_response(FindGroupActivitiesResponse {
                    activities: activity_details,
                }),
            )
        }
        Err(e) => {
            tracing::error!("获取群组活动失败: {}", e);
            (
                StatusCode::OK,
                error_to_api_response::<FindGroupActivitiesResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取群组活动失败: {}", e),
                ),
            )
        }
    }
}

/// 获取所有活动（最新活动）
pub async fn get_all_activities(
    State(state): State<AppState>,
    Query(params): Query<GetAllActivitiesRequest>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20);
    tracing::debug!("请求获取最新的 {} 条活动", limit);

    // 创建活动存储库实例
    let repo = Arc::new(ActivityOperation::new(Arc::new(state.pool.clone())));

    // 获取最新活动
    match repo
        .find_nearby_activities(
            0.0, // 使用0,0作为默认坐标
            0.0,
            100000.0, // 足够大的搜索半径
            limit as i64,
        )
        .await
    {
        Ok(activities) => {
            tracing::debug!("成功获取 {} 条最新活动", activities.len());

            // 将数据库实体转换为API响应格式
            let activity_details: Vec<ActivityDetail> = activities
                .into_iter()
                .map(|activity| ActivityDetail {
                    id: activity.id,
                    activity_type: match activity.activity_type {
                        2 => ActivityType::UserCheckedIn,
                        10 => ActivityType::GroupCreated,
                        11 => ActivityType::UserJoined,
                        20 => ActivityType::MessageSent,
                        _ => ActivityType::UserCheckedIn,
                    },
                    group_id: activity.group_id.unwrap_or_default(),
                    group_name: String::new(),
                    user_id: activity.user_id,
                    user_name: String::new(),
                    description: activity.description,
                    occurred_at: activity.created_at,
                    latitude: activity.latitude,
                    longitude: activity.longitude,
                    distance: None,
                })
                .collect();

            (
                StatusCode::OK,
                success_to_api_response(GetAllActivitiesResponse {
                    activities: activity_details,
                }),
            )
        }
        Err(e) => {
            tracing::error!("获取最新活动失败: {}", e);
            (
                StatusCode::OK,
                error_to_api_response::<GetAllActivitiesResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取最新活动失败: {}", e),
                ),
            )
        }
    }
}

/// 查找附近用户
pub async fn find_nearby_users(
    State(state): State<AppState>,
    Query(params): Query<FindNearbyUsersRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        "请求获取半径 {}m 内的附近用户，坐标: ({}, {})",
        params.radius,
        params.latitude,
        params.longitude
    );

    // 验证坐标是否合法
    if params.latitude < -90.0
        || params.latitude > 90.0
        || params.longitude < -180.0
        || params.longitude > 180.0
    {
        tracing::warn!(
            "非法的地理坐标: 经度={}, 纬度={}",
            params.longitude,
            params.latitude
        );
        return (
            StatusCode::OK,
            error_to_api_response::<FindNearbyUsersResponse>(
                error_codes::VALIDATION_ERROR,
                "非法的地理坐标".to_string(),
            ),
        );
    }

    // 限制搜索半径
    let radius = params.radius as f64;
    let radius = radius.min(state.config.max_search_radius);

    // 创建活动存储库实例
    let db_operation = ActivityOperation::new(Arc::new(state.pool.clone()));

    // 查询附近用户
    match db_operation
        .find_nearby_users(
            params.latitude,
            params.longitude,
            radius,
            params.limit as i64,
        )
        .await
    {
        Ok(nearby_users) => {
            // 转换为API响应格式
            let result: Vec<NearbyUser> = nearby_users
                .into_iter()
                .map(|user| NearbyUser {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    last_activity: UserActivity {
                        id: user.last_activity_id.unwrap_or_default(),
                        activity_type: match user.last_activity_type.as_deref().unwrap_or("") {
                            "USER_CHECKIN" => ActivityType::UserCheckedIn,
                            "GROUP_CREATE" => ActivityType::GroupCreated,
                            "USER_JOINED" => ActivityType::UserJoined,
                            "MESSAGE_SENT" => ActivityType::MessageSent,
                            _ => ActivityType::UserCheckedIn,
                        },
                        description: user.last_activity_description.unwrap_or_default(),
                        occurred_at: user
                            .last_activity_time
                            .unwrap_or_else(|| chrono::Utc::now()),
                    },
                    distance: user.distance,
                })
                .collect();

            (
                StatusCode::OK,
                success_to_api_response(FindNearbyUsersResponse { users: result }),
            )
        }
        Err(e) => {
            tracing::error!("查询附近用户失败: {}", e);
            (
                StatusCode::OK,
                error_to_api_response::<FindNearbyUsersResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("查询附近用户失败: {}", e),
                ),
            )
        }
    }
}
