// 群组处理器
// 处理群组相关的API请求

use crate::api::schema::group::*;
use crate::database::repositories::group::GroupRepository;
use crate::utils::{error_codes, success_to_api_response, error_to_api_response};
use crate::utils::Claims;
use crate::AppState;
use axum::{
    extract::{Extension, Json, Path, Query, State},
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
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 使用仓库方法创建群组
    match repo
        .create_group(
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
            tracing::info!("用户 {} 成功创建群组 {}: {}", claims.sub, group_id, payload.name);
            (
                StatusCode::OK,
                success_to_api_response(CreateGroupResponse { group_id })
            )
        }
        Err(err) => {
            tracing::error!("创建群组失败: {}", err);
            (
                StatusCode::OK,
                error_to_api_response::<CreateGroupResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("创建群组失败: {}", err)
                )
            )
        }
    }
}

/// 按名称搜索群组
pub async fn query_groups_by_name(
    State(state): State<AppState>,
    Query(params): Query<QueryGroupsByNameRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 通过名称查询群组
    match repo.find_groups_by_name(&params.name).await {
        Ok(groups) => {
            // 转换为API响应格式
            let result = groups
                .into_iter()
                .map(|group| NearbyGroup {
                    id: group.id,
                    name: group.name,
                    description: group.description,
                    member_count: 0, // 这里需要从数据库获取真实的成员数量
                    distance: None,  // 按名称搜索时没有距离信息
                })
                .collect();

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => {
            (
                StatusCode::OK,
                error_to_api_response::<Vec<NearbyGroup>>(
                    error_codes::INTERNAL_ERROR,
                    format!("查询群组失败: {}", err)
                )
            )
        }
    }
}

/// 按位置搜索附近群组
pub async fn query_groups_by_location(
    State(state): State<AppState>,
    Query(params): Query<QueryGroupsByLocationRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 搜索附近群组
    match repo
        .find_groups_by_location(params.latitude, params.longitude, params.radius as f64)
        .await
    {
        Ok(groups) => {
            // 转换为API响应格式
            let result = groups
                .into_iter()
                .map(|group| {
                    // 计算距离（简单的欧几里德距离，1度约等于111km）
                    let dx = (group.longitude - params.longitude).abs() * 111000.0;
                    let dy = (group.latitude - params.latitude).abs() * 111000.0;
                    let distance = (dx * dx + dy * dy).sqrt();

                    NearbyGroup {
                        id: group.id,
                        name: group.name,
                        description: group.description,
                        member_count: 0, // 需要从数据库获取真实成员数
                        distance: Some(distance),
                    }
                })
                .collect();

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => {
            (
                StatusCode::OK,
                error_to_api_response::<Vec<NearbyGroup>>(
                    error_codes::INTERNAL_ERROR,
                    format!("搜索附近群组失败: {}", err)
                )
            )
        }
    }
}

/// 加入群组
pub async fn join_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<JoinGroupRequest>,
) -> impl IntoResponse {
    tracing::debug!("用户 {} 正在尝试加入群组 {}", claims.sub, payload.group_id);
    
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 从认证信息中获取用户ID
    let user_id = &claims.sub;

    match repo.add_user_to_group(&payload.group_id, user_id, payload.password.as_deref()).await {
        Ok(_) => {
            tracing::info!("用户 {} 成功加入群组 {}", user_id, payload.group_id);
            (
                StatusCode::OK,
                success_to_api_response(JoinGroupResponse { success: true })
            )
        }
        Err(err) => {
            let error_msg = err.to_string();
            if error_msg.contains("Password required") || error_msg.contains("Invalid password") {
                tracing::warn!("用户 {} 加入群组 {} 失败: 密码错误或缺失", user_id, payload.group_id);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::AUTH_FAILED,
                        "密码错误或缺失".to_string()
                    )
                )
            } else if error_msg.contains("not found") {
                tracing::warn!("用户 {} 加入群组 {} 失败: 群组不存在", user_id, payload.group_id);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::NOT_FOUND,
                        "群组不存在".to_string()
                    )
                )
            } else {
                tracing::error!("用户 {} 加入群组 {} 失败: {}", user_id, payload.group_id, err);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::INTERNAL_ERROR,
                        format!("加入群组失败: {}", err)
                    )
                )
            }
        }
    }
}

/// 获取群组信息
pub async fn get_group_info(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 获取群组信息
    let group_result = repo.find_group_by_id(&group_id).await;

    match group_result {
        Ok(Some(group)) => {
            // 获取成员数量
            match repo.get_member_count(&group_id).await {
                Ok(member_count) => {
                    (
                        StatusCode::OK,
                        success_to_api_response(NearbyGroup {
                            id: group.id,
                            name: group.name,
                            description: group.description,
                            member_count: member_count as u32,
                            distance: None,
                        })
                    )
                }
                Err(err) => {
                    tracing::error!("获取群组 {} 成员数量失败: {}", group_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<NearbyGroup>(
                            error_codes::INTERNAL_ERROR,
                            format!("获取群组成员数量失败: {}", err)
                        )
                    )
                }
            }
        }
        Ok(None) => {
            (
                StatusCode::OK,
                error_to_api_response::<NearbyGroup>(
                    error_codes::NOT_FOUND,
                    "群组不存在".to_string()
                )
            )
        }
        Err(err) => {
            tracing::error!("获取群组 {} 信息失败: {}", group_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<NearbyGroup>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取群组信息失败: {}", err)
                )
            )
        }
    }
}

/// 获取群组成员
pub async fn get_group_members(
    State(state): State<AppState>,
    Path(group_id): Path<String>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 检查群组是否存在
    match repo.group_exists(&group_id).await {
        Ok(exists) => {
            if !exists {
                return (
                    StatusCode::OK,
                    error_to_api_response::<Vec<GroupMember>>(
                        error_codes::NOT_FOUND,
                        "群组不存在".to_string()
                    )
                );
            }
            
            // 获取群组成员列表
            match repo.get_group_members(&group_id).await {
                Ok(members) => {
                    // 转换为API响应格式
                    let result = members
                        .into_iter()
                        .map(|(user_id, nickname, last_active)| GroupMember {
                            user_id,
                            nickname,
                            last_active,
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
                            format!("获取群组成员失败: {}", err)
                        )
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
                    format!("检查群组失败: {}", err)
                )
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
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    let user_id = &claims.sub;
    tracing::debug!("用户 {} 正在尝试离开群组 {}", user_id, group_id);

    // 检查用户是否属于群组
    match repo.user_in_group(&group_id, user_id).await {
        Ok(is_member) => {
            if !is_member {
                tracing::warn!("用户 {} 尝试离开未加入的群组 {}", user_id, group_id);
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::VALIDATION_ERROR,
                        "用户不在该群组中".to_string()
                    )
                );
            }
            
            // 用户离开群组
            match repo.remove_user_from_group(&group_id, user_id).await {
                Ok(_) => {
                    tracing::info!("用户 {} 成功离开群组 {}", user_id, group_id);
                    (
                        StatusCode::OK,
                        success_to_api_response(JoinGroupResponse { success: true })
                    )
                }
                Err(err) => {
                    tracing::error!("用户 {} 离开群组 {} 失败: {}", user_id, group_id, err);
                    (
                        StatusCode::OK,
                        error_to_api_response::<JoinGroupResponse>(
                            error_codes::INTERNAL_ERROR,
                            format!("离开群组失败: {}", err)
                        )
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
                    format!("检查用户群组成员身份失败: {}", err)
                )
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
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    let user_id = &claims.sub;
    tracing::debug!("更新用户 {} 在群组 {} 中的活跃状态", user_id, group_id);

    // 更新用户在群组中的最后活跃时间
    match repo.update_user_activity(&group_id, user_id).await {
        Ok(_) => {
            tracing::debug!("用户 {} 在群组 {} 中的活跃状态已更新", user_id, group_id);
            (
                StatusCode::OK,
                success_to_api_response(JoinGroupResponse { success: true })
            )
        }
        Err(err) => {
            let error_msg = err.to_string();
            if error_msg.contains("not found") {
                tracing::warn!("用户 {} 在群组 {} 中的活跃状态更新失败: 群组不存在或用户不在群组中", user_id, group_id);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::NOT_FOUND,
                        "群组不存在或用户不在群组中".to_string()
                    )
                )
            } else {
                tracing::error!("用户 {} 在群组 {} 中的活跃状态更新失败: {}", user_id, group_id, err);
                (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::INTERNAL_ERROR,
                        format!("更新活跃状态失败: {}", err)
                    )
                )
            }
        }
    }
}

/// 查询附近群组
pub async fn find_nearby_groups(
    State(state): State<AppState>,
    Query(params): Query<QueryGroupsByLocationRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    // 与query_groups_by_location功能相同，但是专门用于查找附近群组
    match repo
        .find_groups_by_location(params.latitude, params.longitude, params.radius as f64)
        .await
    {
        Ok(groups) => {
            // 转换为API响应格式
            let result = groups
                .into_iter()
                .map(|group| {
                    // 计算距离（简单的欧几里德距离，1度约等于111km）
                    let dx = (group.longitude - params.longitude).abs() * 111000.0;
                    let dy = (group.latitude - params.latitude).abs() * 111000.0;
                    let distance = (dx * dx + dy * dy).sqrt();

                    NearbyGroup {
                        id: group.id,
                        name: group.name,
                        description: group.description,
                        member_count: 0, // 需要从数据库获取真实成员数
                        distance: Some(distance),
                    }
                })
                .collect();

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => {
            (
                StatusCode::OK,
                error_to_api_response::<Vec<NearbyGroup>>(
                    error_codes::INTERNAL_ERROR,
                    format!("搜索附近群组失败: {}", err)
                )
            )
        }
    }
}

/// 获取用户所在的所有群组
pub async fn get_user_groups(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    let user_id = &claims.sub;
    tracing::debug!("获取用户 {} 加入的所有群组", user_id);

    // 获取用户加入的所有群组
    match repo.find_groups_by_user_id(user_id).await {
        Ok(groups) => {
            tracing::debug!("用户 {} 已加入 {} 个群组", user_id, groups.len());
            
            // 转换为API响应格式
            let result = groups
                .into_iter()
                .map(|group| NearbyGroup {
                    id: group.id,
                    name: group.name,
                    description: group.description,
                    member_count: 0, // 需要从数据库获取真实成员数
                    distance: None,  // 用户的群组列表不需要距离信息
                })
                .collect();

            (StatusCode::OK, success_to_api_response(result))
        }
        Err(err) => {
            tracing::error!("获取用户 {} 群组列表失败: {}", user_id, err);
            (
                StatusCode::OK,
                error_to_api_response::<Vec<NearbyGroup>>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取用户群组失败: {}", err)
                )
            )
        }
    }
}

/// 从群组中移除成员
pub async fn remove_group_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((group_id, user_id)): Path<(String, String)>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    let current_user_id = &claims.sub;
    tracing::debug!("用户 {} 尝试从群组 {} 中移除成员 {}", current_user_id, group_id, user_id);

    // 检查当前用户是否是群组管理员
    match repo.user_is_admin(&group_id, current_user_id).await {
        Ok(is_admin) => {
            if !is_admin {
                tracing::warn!("用户 {} 尝试移除群组 {} 的成员 {} 但没有管理员权限", 
                    current_user_id, group_id, user_id);
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::PERMISSION_DENIED,
                        "只有管理员可以移除成员".to_string()
                    )
                );
            }
            
            // 检查要移除的用户是否在群组中
            match repo.user_in_group(&group_id, &user_id).await {
                Ok(is_member) => {
                    if !is_member {
                        tracing::warn!("用户 {} 尝试从群组 {} 中移除不存在的成员 {}", 
                            current_user_id, group_id, user_id);
                        return (
                            StatusCode::OK,
                            error_to_api_response::<JoinGroupResponse>(
                                error_codes::VALIDATION_ERROR,
                                "该用户不在群组中".to_string()
                            )
                        );
                    }
                    
                    // 移除用户
                    match repo.remove_user_from_group(&group_id, &user_id).await {
                        Ok(_) => {
                            tracing::info!("用户 {} 成功从群组 {} 中移除成员 {}", current_user_id, group_id, user_id);
                            (
                                StatusCode::OK,
                                success_to_api_response(JoinGroupResponse { success: true })
                            )
                        }
                        Err(err) => {
                            tracing::error!("从群组 {} 中移除成员 {} 失败: {}", group_id, user_id, err);
                            (
                                StatusCode::OK,
                                error_to_api_response::<JoinGroupResponse>(
                                    error_codes::INTERNAL_ERROR,
                                    format!("移除群组成员失败: {}", err)
                                )
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
                            format!("检查用户群组成员身份失败: {}", err)
                        )
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
                    format!("检查管理员权限失败: {}", err)
                )
            )
        }
    }
}

/// 设置群组成员角色
pub async fn update_user_role(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((group_id, user_id)): Path<(String, String)>,
    Json(payload): Json<SetMemberRoleRequest>,
) -> impl IntoResponse {
    // 创建仓库实例
    let repo = GroupRepository::new(Arc::new(state.pool.clone()));
    
    let current_user_id = &claims.sub;
    tracing::debug!("用户 {} 尝试设置群组 {} 的成员 {} 角色", 
        current_user_id, group_id, user_id);

    // 检查当前用户是否是群组管理员
    match repo.user_is_admin(&group_id, current_user_id).await {
        Ok(is_admin) => {
            if !is_admin {
                tracing::warn!("用户 {} 尝试修改群组 {} 的成员 {} 角色但没有管理员权限", 
                    current_user_id, group_id, user_id);
                return (
                    StatusCode::OK,
                    error_to_api_response::<JoinGroupResponse>(
                        error_codes::PERMISSION_DENIED,
                        "只有管理员可以修改成员角色".to_string()
                    )
                );
            }
            
            // 检查要修改角色的用户是否在群组中
            match repo.user_in_group(&group_id, &user_id).await {
                Ok(is_member) => {
                    if !is_member {
                        tracing::warn!("用户 {} 尝试修改群组 {} 中不存在的成员 {} 的角色", 
                            current_user_id, group_id, user_id);
                        return (
                            StatusCode::OK,
                            error_to_api_response::<JoinGroupResponse>(
                                error_codes::VALIDATION_ERROR,
                                "该用户不在群组中".to_string()
                            )
                        );
                    }
                    
                    // 修改用户角色
                    match repo.update_user_role(&group_id, &user_id, payload.is_admin).await {
                        Ok(_) => {
                            tracing::info!("用户 {} 成功设置群组 {} 的成员 {} 角色为: {}", 
                                current_user_id, group_id, user_id, if payload.is_admin { "admin" } else { "member" });
                            (
                                StatusCode::OK,
                                success_to_api_response(JoinGroupResponse { success: true })
                            )
                        }
                        Err(err) => {
                            tracing::error!("修改群组 {} 的成员 {} 角色失败: {}", group_id, user_id, err);
                            (
                                StatusCode::OK,
                                error_to_api_response::<JoinGroupResponse>(
                                    error_codes::INTERNAL_ERROR,
                                    format!("修改成员角色失败: {}", err)
                                )
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
                            format!("检查用户群组成员身份失败: {}", err)
                        )
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
                    format!("检查管理员权限失败: {}", err)
                )
            )
        }
    }
}
