// 群组处理器
// 处理群组相关的API请求

use crate::AppState;
use crate::api::models::group::*;
use crate::database::operations::group::GroupOperation;
use crate::utils::Claims;
use crate::utils::{error_codes, error_to_api_response, success_to_api_response};
use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

/// 创建群组
pub async fn create_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    tracing::debug!("用户 {} 正在创建群组: {}", claims.sub, payload.name);

    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    // 使用仓库方法创建群组
    match repo
        .create(
            &payload.name,
            &payload.location_name,
            payload.latitude,
            payload.longitude,
            &payload.description.unwrap_or_default(),
            payload.password.as_deref(),
            &claims.sub, // 使用认证信息中的用户ID
        )
        .await
    {
        Ok(group_id) => {
            tracing::info!(
                "用户 {} 成功创建群组 {}: {}",
                claims.sub,
                group_id,
                payload.name
            );
            (
                StatusCode::OK,
                success_to_api_response(CreateGroupResponse { group_id }),
            )
        }
        Err(err) => {
            tracing::error!("创建群组失败: {}", err);
            (
                StatusCode::OK,
                error_to_api_response::<CreateGroupResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("创建群组失败: {}", err),
                ),
            )
        }
    }
}

/// 获取群组信息（路径参数）
pub async fn get_group_info(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    // 获取群组信息
    match repo.find_by_id_with_creator(&group_id).await {
        Ok(Some((group, creator))) => {
            // 获取成员数量
            match repo.count_members(&group_id).await {
                Ok(member_count) => {
                    // 构建详细的群组信息
                    let detailed_group = GroupDetail {
                        group_id: group.id,
                        name: group.name,
                        description: group.description,
                        public_creator_id: creator.public_user_id,
                        creator_name: Some(creator.nickname),
                        avatar_url: None, // 需要额外查询创建者头像
                        created_at: group.created_at,
                        last_active_at: group.last_active,
                        latitude: group.latitude,
                        longitude: group.longitude,
                        member_count,
                        distance: 0.0, // 单个群组查询不需要距离
                        location_name: group.location_name,
                        is_password_required: group.password.is_some(),
                    };

                    (StatusCode::OK, success_to_api_response(detailed_group))
                }
                Err(err) => {
                    tracing::error!("获取群组 {} 成员数量失败: {}", group_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<GroupDetail>(
                            error_codes::INTERNAL_ERROR,
                            format!("获取群组成员数量失败: {}", err),
                        ),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::OK,
            error_to_api_response::<GroupDetail>(error_codes::NOT_FOUND, "群组不存在".to_string()),
        ),
        Err(err) => {
            tracing::error!("获取群组 {} 信息失败: {}", group_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<GroupDetail>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取群组信息失败: {}", err),
                ),
            )
        }
    }
}

/// 获取群组成员列表
pub async fn get_group_members(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let user_id = &claims.sub;
    tracing::debug!("用户 {} 正在获取群组 {} 的成员列表", user_id, group_id);

    // 首先检查群组是否存在
    match repo.exists(&group_id).await {
        Ok(exists) => {
            if !exists {
                tracing::warn!("用户 {} 尝试获取不存在的群组 {} 的成员", user_id, group_id);
                return (
                    StatusCode::OK,
                    error_to_api_response::<Vec<GroupMember>>(
                        error_codes::NOT_FOUND,
                        "群组不存在".to_string(),
                    ),
                );
            }

            // 获取成员列表
            match repo.get_members_with_public_id(&group_id).await {
                Ok(members) => {
                    // 转换为API响应格式
                    let result = members
                        .into_iter()
                        .map(|(_user_id, nickname, last_active, public_user_id, role)| {
                            GroupMember {
                                public_user_id,
                                nickname,
                                last_active,
                                role: role.unwrap_or_else(|| "member".to_string()),
                            }
                        })
                        .collect();

                    (StatusCode::OK, success_to_api_response(result))
                }
                Err(err) => {
                    tracing::error!("获取群组 {} 成员失败: {}", group_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<Vec<GroupMember>>(
                            error_codes::INTERNAL_ERROR,
                            format!("获取群组成员失败: {}", err),
                        ),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!("检查群组 {} 是否存在失败: {}", group_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<Vec<GroupMember>>(
                    error_codes::INTERNAL_ERROR,
                    format!("检查群组是否存在失败: {}", err),
                ),
            )
        }
    }
}

/// 离开群组
pub async fn leave_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let user_id = &claims.sub;
    tracing::debug!("用户 {} 正在尝试离开群组 {}", user_id, group_id);

    // 检查用户是否属于群组
    match repo.has_user(&group_id, user_id).await {
        Ok(is_member) => {
            if !is_member {
                tracing::warn!("用户 {} 尝试离开未加入的群组 {}", user_id, group_id);
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::VALIDATION_ERROR,
                        "用户不在该群组中".to_string(),
                    ),
                );
            }

            // 用户离开群组
            match repo.remove_user(&group_id, user_id).await {
                Ok(_) => {
                    tracing::info!("用户 {} 成功离开群组 {}", user_id, group_id);
                    (
                        StatusCode::OK,
                        success_to_api_response(JoinGroupResponse { success: true }),
                    )
                }
                Err(err) => {
                    tracing::error!("用户 {} 离开群组 {} 失败: {}", user_id, group_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<JoinGroupResponse>(
                            error_codes::INTERNAL_ERROR,
                            format!("离开群组失败: {}", err),
                        ),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!("检查用户 {} 群组成员身份失败: {}", user_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<JoinGroupResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("检查用户成员身份失败: {}", err),
                ),
            )
        }
    }
}

/// 保持群组活跃
pub async fn keep_alive(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let user_id = &claims.sub;
    tracing::debug!("用户 {} 正在保持群组 {} 活跃", user_id, group_id);

    // 更新用户在群组中的活跃状态
    match repo.update_user_activity(&group_id, user_id).await {
        Ok(_) => {
            tracing::debug!("用户 {} 在群组 {} 中的活跃状态已更新", user_id, group_id);
            (
                StatusCode::OK,
                success_to_api_response(JoinGroupResponse { success: true }),
            )
        }
        Err(err) => {
            let error_msg = err.to_string();
            if error_msg.contains("not found") {
                tracing::warn!(
                    "用户 {} 在群组 {} 中的活跃状态更新失败: 群组不存在或用户不在群组中",
                    user_id,
                    group_id
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::NOT_FOUND,
                        "群组不存在或用户不在群组中".to_string(),
                    ),
                )
            } else {
                tracing::error!(
                    "用户 {} 在群组 {} 中的活跃状态更新失败: {}",
                    user_id,
                    group_id,
                    err
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::INTERNAL_ERROR,
                        format!("更新活跃状态失败: {}", err),
                    ),
                )
            }
        }
    }
}

/// 获取用户的所有群组
pub async fn get_user_groups(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let user_id = &claims.sub;
    tracing::debug!("获取用户 {} 加入的所有群组", user_id);

    // 获取用户加入的所有群组
    match repo.find_by_user_id_with_creators(user_id).await {
        Ok(groups) => {
            tracing::debug!("用户 {} 已加入 {} 个群组", user_id, groups.len());

            // 转换为API响应格式
            let mut result = Vec::new();

            for (group, creator) in groups {
                result.push(GroupDetail {
                    group_id: group.id,
                    name: group.name,
                    description: group.description,
                    public_creator_id: creator.public_user_id,
                    creator_name: Some(creator.nickname),
                    avatar_url: None, // 需要额外查询
                    created_at: group.created_at,
                    last_active_at: group.last_active,
                    latitude: group.latitude,
                    longitude: group.longitude,
                    member_count: 0, // 需要从数据库获取真实成员数
                    distance: 0.0,   // 用户的群组列表不需要距离信息
                    location_name: group.location_name,
                    is_password_required: group.password.is_some(),
                });
            }

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => {
            tracing::error!("获取用户 {} 群组列表失败: {}", user_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<Vec<GroupDetail>>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取用户群组失败: {}", err),
                ),
            )
        }
    }
}

/// 移除群组成员（需要管理员权限）
pub async fn remove_group_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((group_id, target_user_id)): Path<(String, String)>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let current_user_id = &claims.sub;
    tracing::debug!(
        "用户 {} 尝试从群组 {} 中移除用户 {}",
        current_user_id,
        group_id,
        target_user_id
    );

    // 检查当前用户是否为管理员
    match repo.is_admin(&group_id, current_user_id).await {
        Ok(is_admin) => {
            if !is_admin {
                tracing::warn!(
                    "用户 {} 尝试无权限操作: 从群组 {} 移除用户 {}",
                    current_user_id,
                    group_id,
                    target_user_id
                );
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::PERMISSION_DENIED,
                        "需要管理员权限".to_string(),
                    ),
                );
            }

            // 检查目标用户是否在群组中
            match repo.has_user(&group_id, &target_user_id).await {
                Ok(is_member) => {
                    if !is_member {
                        tracing::warn!(
                            "用户 {} 尝试移除不存在的成员 {}",
                            current_user_id,
                            target_user_id
                        );
                        return (
                            StatusCode::OK,
                            error_to_api_response::<JoinGroupResponse>(
                                error_codes::NOT_FOUND,
                                "目标用户不在群组中".to_string(),
                            ),
                        );
                    }

                    // 移除目标用户
                    match repo.remove_user(&group_id, &target_user_id).await {
                        Ok(_) => {
                            tracing::info!(
                                "用户 {} 成功从群组 {} 中移除成员 {}",
                                current_user_id,
                                group_id,
                                target_user_id
                            );
                            (
                                StatusCode::OK,
                                success_to_api_response(JoinGroupResponse { success: true }),
                            )
                        }
                        Err(err) => {
                            tracing::error!(
                                "从群组 {} 中移除成员 {} 失败: {}",
                                group_id,
                                target_user_id,
                                err
                            );
                            (
                                StatusCode::OK,
                                error_to_api_response::<JoinGroupResponse>(
                                    error_codes::INTERNAL_ERROR,
                                    format!("移除群组成员失败: {}", err),
                                ),
                            )
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("检查用户 {} 群组成员身份失败: {}", target_user_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<JoinGroupResponse>(
                            error_codes::INTERNAL_ERROR,
                            format!("检查群组成员身份失败: {}", err),
                        ),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!("检查用户 {} 的管理员权限失败: {}", current_user_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<JoinGroupResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("检查管理员权限失败: {}", err),
                ),
            )
        }
    }
}

/// 群组名称搜索
pub async fn search_groups_by_name(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    tracing::debug!("用户 {} 正在搜索群组名称: {}", claims.sub, name);

    // 搜索群组
    match repo.find_by_name_with_creator(&name).await {
        Ok(groups) => {
            // 转换为API响应格式
            let mut result = Vec::new();

            for (group, creator) in groups {
                result.push(GroupDetail {
                    group_id: group.id,
                    name: group.name,
                    description: group.description,
                    public_creator_id: creator.public_user_id,
                    creator_name: Some(creator.nickname),
                    avatar_url: None,
                    created_at: group.created_at,
                    last_active_at: group.last_active,
                    latitude: group.latitude,
                    longitude: group.longitude,
                    member_count: 0,
                    distance: 0.0,
                    location_name: group.location_name,
                    is_password_required: group.password.is_some(),
                });
            }

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => (
            StatusCode::OK,
            error_to_api_response::<Vec<GroupDetail>>(
                error_codes::INTERNAL_ERROR,
                format!("查询群组失败: {}", err),
            ),
        ),
    }
}

/// 更新用户在群组中的角色
pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((group_id, target_user_id)): Path<(String, String)>,
    Json(payload): Json<SetMemberRoleRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    let current_user_id = &claims.sub;
    tracing::debug!(
        "用户 {} 尝试更新用户 {} 在群组 {} 中的角色为 {}",
        current_user_id,
        target_user_id,
        group_id,
        payload.role
    );

    // 检查当前用户是否为管理员
    match repo.is_admin(&group_id, current_user_id).await {
        Ok(is_admin) => {
            if !is_admin {
                tracing::warn!(
                    "用户 {} 尝试无权限操作: 修改群组 {} 的成员 {} 角色但没有管理员权限",
                    current_user_id,
                    group_id,
                    target_user_id
                );
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::PERMISSION_DENIED,
                        "需要管理员权限".to_string(),
                    ),
                );
            }

            // 检查要修改角色的用户是否在群组中
            match repo.has_user(&group_id, &target_user_id).await {
                Ok(is_member) => {
                    if !is_member {
                        tracing::warn!(
                            "用户 {} 尝试修改群组 {} 中不存在的成员 {} 的角色",
                            current_user_id,
                            group_id,
                            target_user_id
                        );
                        return (
                            StatusCode::OK,
                            error_to_api_response::<JoinGroupResponse>(
                                error_codes::NOT_FOUND,
                                "该用户不在群组中".to_string(),
                            ),
                        );
                    }

                    // 修改用户角色
                    match repo
                        .update_user_role(&group_id, &target_user_id, payload.role == "admin")
                        .await
                    {
                        Ok(_) => {
                            tracing::info!(
                                "用户 {} 成功设置群组 {} 的成员 {} 角色为: {}",
                                current_user_id,
                                group_id,
                                target_user_id,
                                payload.role
                            );
                            (
                                StatusCode::OK,
                                success_to_api_response(JoinGroupResponse { success: true }),
                            )
                        }
                        Err(err) => {
                            tracing::error!(
                                "修改群组 {} 的成员 {} 角色失败: {}",
                                group_id,
                                target_user_id,
                                err
                            );
                            (
                                StatusCode::OK,
                                error_to_api_response::<JoinGroupResponse>(
                                    error_codes::INTERNAL_ERROR,
                                    format!("修改成员角色失败: {}", err),
                                ),
                            )
                        }
                    }
                }
                Err(err) => {
                    tracing::error!("检查用户 {} 群组成员身份失败: {}", target_user_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<JoinGroupResponse>(
                            error_codes::INTERNAL_ERROR,
                            format!("检查群组成员身份失败: {}", err),
                        ),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!("检查用户 {} 的管理员权限失败: {}", current_user_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<JoinGroupResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("检查管理员权限失败: {}", err),
                ),
            )
        }
    }
}

/// 位置搜索附近群组
pub async fn search_groups_by_location(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<NearbyGroupsRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    tracing::debug!(
        "用户 {} 正在搜索位置附近的群组: [{}, {}], 半径: {}米",
        claims.sub,
        payload.latitude,
        payload.longitude,
        payload.radius
    );

    // 搜索附近群组
    match repo
        .find_by_location_with_details(payload.latitude, payload.longitude, payload.radius as f64)
        .await
    {
        Ok(groups) => {
            let mut detailed_groups = Vec::new();

            for group in groups {
                // 计算距离
                let dx = (group.longitude - payload.longitude).abs() * 111000.0;
                let dy = (group.latitude - payload.latitude).abs() * 111000.0;
                let distance = (dx * dx + dy * dy).sqrt();

                // 检查群组是否需要密码
                let group_info = repo.find_by_id(&group.id).await;
                let is_password_required = match group_info {
                    Ok(Some(info)) => info.password.is_some(),
                    _ => false, // 默认不需要密码
                };

                detailed_groups.push(GroupDetail {
                    group_id: group.id,
                    name: group.name,
                    description: group.description,
                    public_creator_id: group.creator_public_id.unwrap_or_default(),
                    creator_name: group.creator_name,
                    avatar_url: group.avatar_url,
                    created_at: group.created_at,
                    last_active_at: group.last_active_at,
                    latitude: group.latitude,
                    longitude: group.longitude,
                    member_count: group.member_count,
                    distance,
                    location_name: group.location_name,
                    is_password_required,
                });
            }

            (StatusCode::OK, success_to_api_response(detailed_groups))
        }
        Err(err) => {
            tracing::error!("查找附近群组失败: {}", err);
            (
                StatusCode::OK,
                error_to_api_response::<Vec<GroupDetail>>(
                    error_codes::INTERNAL_ERROR,
                    format!("搜索附近群组失败: {}", err),
                ),
            )
        }
    }
}

/// 加入群组
pub async fn join_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(group_id): Path<String>,
    Json(payload): Json<JoinGroupWithPasswordRequest>,
) -> impl IntoResponse {
    tracing::debug!("用户 {} 正在尝试加入群组 {}", claims.sub, group_id);

    // 创建仓库实例
    let repo = GroupOperation::new(Arc::new(state.pool.clone()));

    // 从认证信息中获取用户ID
    let user_id = &claims.sub;

    // 添加用户到群组
    match repo
        .add_user(&group_id, user_id, payload.password.as_deref())
        .await
    {
        Ok(_) => {
            tracing::info!("用户 {} 成功加入群组 {}", user_id, group_id);
            (
                StatusCode::OK,
                success_to_api_response(JoinGroupResponse { success: true }),
            )
        }
        Err(err) => {
            let error_msg = err.to_string();
            if error_msg.contains("Password required") || error_msg.contains("Invalid password") {
                tracing::warn!(
                    "用户 {} 加入群组 {} 失败: 密码错误或缺失",
                    user_id,
                    group_id
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::AUTH_FAILED,
                        "密码错误或缺失".to_string(),
                    ),
                )
            } else if error_msg.contains("not found") {
                tracing::warn!("用户 {} 加入群组 {} 失败: 群组不存在", user_id, group_id);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::NOT_FOUND,
                        "群组不存在".to_string(),
                    ),
                )
            } else {
                tracing::error!("用户 {} 加入群组 {} 失败: {}", user_id, group_id, err);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::INTERNAL_ERROR,
                        format!("加入群组失败: {}", err),
                    ),
                )
            }
        }
    }
}
