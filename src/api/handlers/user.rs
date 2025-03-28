use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    utils::{
        Claims, error_codes, error_to_api_response, generate_temp_token, generate_token,
        success_to_api_response, verify_password,
    },
    api::schema::user::{
        CreateRegisteredUserRequest, CreateTemporaryUserRequest, CreateUserResponse,
        LoginRequest, LoginResponse, ResetPasswordRequest, ResetPasswordResponse,
        UpdateNicknameRequest, UpdatePasswordRequest, RefreshTokenResponse, CheckTokenResponse, UserInfo
    },
    database::repositories::user::UserRepository,
    cache::operations::user::UserCacheOperations,
};

/// 注册新用户
#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<CreateRegisteredUserRequest>,
) -> impl IntoResponse {
    // 检查用户ID格式
    if !req.user_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return (
            StatusCode::OK,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "用户ID格式无效，只允许使用字母、数字和下划线".to_string(),
            ),
        );
    }

    // 创建用户
    match UserRepository::create_registered_user(
        &state.pool, 
        &req.user_id, 
        &req.nickname, 
        &req.password
    ).await {
        Ok(user) => {
            // 缓存用户信息
            if let Err(e) = UserCacheOperations::cache_user(&state.redis, &user).await {
                tracing::warn!("Failed to cache user: {}", e);
            }

            // 生成 token
            match generate_token(&user.user_id, &state.config) {
                Ok((token, expires_at)) => (
                    StatusCode::OK,
                    success_to_api_response(CreateUserResponse {
                        user_id: user.user_id.clone(),
                        nickname: user.nickname.clone(),
                        token,
                        expires_at: Some(expires_at),
                    }),
                ),
                Err(_) => (
                    StatusCode::OK,
                    error_to_api_response(error_codes::INTERNAL_ERROR, "生成令牌失败".to_string()),
                ),
            }
        }
        Err(e) => {
            if e.to_string().contains("unique constraint") {
                (
                    StatusCode::OK,
                    error_to_api_response(error_codes::USER_EXISTS, "用户已存在".to_string()),
                )
            } else {
                (
                    StatusCode::OK,
                    error_to_api_response(error_codes::INTERNAL_ERROR, "创建用户失败".to_string()),
                )
            }
        }
    }
}

/// 创建临时用户
#[axum::debug_handler]
pub async fn create_temporary(
    State(state): State<AppState>,
    Json(req): Json<CreateTemporaryUserRequest>,
) -> impl IntoResponse {
    // 生成随机用户ID和昵称
    let user_id = Uuid::new_v4().to_string();
    let nickname = req.nickname.unwrap_or_else(|| format!("用户{}", &user_id[0..6]));

    // 创建临时用户
    match UserRepository::create_temporary_user(&state.pool, &user_id, &nickname).await {
        Ok(user) => {
            // 缓存用户信息
            if let Err(e) = UserCacheOperations::cache_user(&state.redis, &user).await {
                tracing::warn!("Failed to cache temporary user: {}", e);
            }

            // 生成临时token
            match generate_temp_token(&user.user_id, &state.config) {
                Ok((token, expires_at)) => (
                    StatusCode::OK,
                    success_to_api_response(CreateUserResponse {
                        user_id: user.user_id.clone(),
                        nickname: user.nickname.clone(),
                        token,
                        expires_at: Some(expires_at),
                    }),
                ),
                Err(_) => (
                    StatusCode::OK,
                    error_to_api_response(error_codes::INTERNAL_ERROR, "生成临时令牌失败".to_string()),
                ),
            }
        }
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "创建临时用户失败".to_string()),
        ),
    }
}

/// 用户登录
#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // 先尝试从缓存获取用户
    let user_from_db = match UserCacheOperations::get_cached_user(&state.redis, &req.user_id).await {
        Ok(Some(cached_user)) => {
            // 临时用户不能使用密码登录
            if cached_user.is_temporary {
                return (
                    StatusCode::OK,
                    error_to_api_response(
                        error_codes::AUTH_FAILED,
                        "临时用户不能使用密码登录".to_string(),
                    ),
                );
            }
            
            // 缓存中没有密码信息，需要从数据库获取
            match UserRepository::find_by_id(&state.pool, &req.user_id).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    return (
                        StatusCode::OK,
                        error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
                    );
                }
                Err(_) => {
                    return (
                        StatusCode::OK,
                        error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
                    );
                }
            }
        }
        Ok(None) | Err(_) => {
            // 缓存中没有，从数据库获取
            match UserRepository::find_by_id(&state.pool, &req.user_id).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    return (
                        StatusCode::OK,
                        error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
                    );
                }
                Err(_) => {
                    return (
                        StatusCode::OK,
                        error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
                    );
                }
            }
        }
    };

    // 检查是否为临时用户
    if user_from_db.is_temporary {
        return (
            StatusCode::OK,
            error_to_api_response(
                error_codes::AUTH_FAILED,
                "临时用户不能使用密码登录".to_string(),
            ),
        );
    }

    // 验证密码
    let password_valid = match &user_from_db.password_hash {
        Some(hash) => match verify_password(&req.password, hash) {
            Ok(valid) => valid,
            Err(_) => {
                return (
                    StatusCode::OK,
                    error_to_api_response(error_codes::INTERNAL_ERROR, "密码验证失败".to_string()),
                );
            }
        },
        None => false,
    };

    if !password_valid {
        return (
            StatusCode::OK,
            error_to_api_response(error_codes::AUTH_FAILED, "密码无效".to_string()),
        );
    }

    // 更新用户状态为在线
    if let Err(e) = UserCacheOperations::update_user_status(&state.redis, &req.user_id, true, None).await {
        tracing::warn!("Failed to update user status: {}", e);
    }

    // 生成 token
    match generate_token(&user_from_db.user_id, &state.config) {
        Ok((token, expires_at)) => (
            StatusCode::OK,
            success_to_api_response(LoginResponse {
                user_id: user_from_db.user_id,
                nickname: user_from_db.nickname,
                token,
                expires_at: Some(expires_at),
            }),
        ),
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "生成令牌失败".to_string()),
        ),
    }
}

/// 更新用户昵称
#[axum::debug_handler]
pub async fn update_nickname(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdateNicknameRequest>,
) -> impl IntoResponse {
    // 验证昵称长度
    if req.nickname.len() < 2 || req.nickname.len() > 24 {
        return (
            StatusCode::BAD_REQUEST,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "昵称长度必须在2-24个字符之间".to_string(),
            ),
        );
    }

    // 更新昵称
    match UserRepository::update_nickname(&state.pool, &claims.sub, &req.nickname).await {
        Ok(user) => {
            // 更新缓存
            if let Err(e) = UserCacheOperations::cache_user(&state.redis, &user).await {
                tracing::warn!("Failed to update user cache: {}", e);
            }

            (
                StatusCode::OK,
                success_to_api_response(UserInfo {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    is_temporary: user.is_temporary,
                    created_at: Some(user.created_at),
                    updated_at: None,
                    avatar_url: None,
                }),
            )
        }
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "更新昵称失败".to_string()),
        ),
    }
}

/// 更新用户密码
#[axum::debug_handler]
pub async fn update_password(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<UpdatePasswordRequest>,
) -> impl IntoResponse {
    // 从数据库获取用户信息
    let user = match UserRepository::find_by_id(&state.pool, &claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
            );
        }
        Err(_) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
            );
        }
    };

    // 临时用户不能设置密码
    if user.is_temporary {
        return (
            StatusCode::OK,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "临时用户不能设置密码".to_string(),
            ),
        );
    }

    // 验证旧密码
    if let Some(password_hash) = user.password_hash {
        match verify_password(&req.old_password, &password_hash) {
            Ok(true) => {
                // 更新密码
                match UserRepository::update_password(&state.pool, &claims.sub, &req.new_password).await {
                    Ok(_) => (
                        StatusCode::OK,
                        success_to_api_response(true),
                    ),
                    Err(_) => (
                        StatusCode::OK,
                        error_to_api_response(error_codes::INTERNAL_ERROR, "更新密码失败".to_string()),
                    ),
                }
            }
            _ => (
                StatusCode::OK,
                error_to_api_response(error_codes::AUTH_FAILED, "旧密码验证失败".to_string()),
            ),
        }
    } else {
        (
            StatusCode::OK,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "用户没有设置密码".to_string(),
            ),
        )
    }
}

/// 重置密码
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    // 从数据库获取用户信息
    let user = match UserRepository::find_by_id(&state.pool, &req.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
            );
        }
        Err(_) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
            );
        }
    };

    // 临时用户不能重置密码
    if user.is_temporary {
        return (
            StatusCode::OK,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "临时用户不能重置密码".to_string(),
            ),
        );
    }

    // 验证重置码（实际项目中应当实现真正的验证逻辑）
    if req.reset_code != "000000" {
        return (
            StatusCode::OK,
            error_to_api_response(error_codes::VALIDATION_ERROR, "重置码无效".to_string()),
        );
    }

    // 更新密码
    match UserRepository::update_password(&state.pool, &req.user_id, &req.new_password).await {
        Ok(_) => (
            StatusCode::OK,
            success_to_api_response(ResetPasswordResponse { success: true }),
        ),
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "重置密码失败".to_string()),
        ),
    }
}

/// 刷新令牌
#[axum::debug_handler]
pub async fn refresh_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // 检查用户是否存在
    match UserRepository::find_by_id(&state.pool, &claims.sub).await {
        Ok(Some(user)) => {
            // 生成新令牌
            if user.is_temporary {
                match generate_temp_token(&user.user_id, &state.config) {
                    Ok((token, expires_at)) => (
                        StatusCode::OK,
                        success_to_api_response(RefreshTokenResponse {
                            token,
                            expires_at: Some(expires_at),
                        }),
                    ),
                    Err(_) => (
                        StatusCode::OK,
                        error_to_api_response(error_codes::INTERNAL_ERROR, "生成临时令牌失败".to_string()),
                    ),
                }
            } else {
                match generate_token(&user.user_id, &state.config) {
                    Ok((token, expires_at)) => (
                        StatusCode::OK,
                        success_to_api_response(RefreshTokenResponse {
                            token,
                            expires_at: Some(expires_at),
                        }),
                    ),
                    Err(_) => (
                        StatusCode::OK,
                        error_to_api_response(error_codes::INTERNAL_ERROR, "生成令牌失败".to_string()),
                    ),
                }
            }
        }
        Ok(None) => (
            StatusCode::OK,
            error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
        ),
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
        ),
    }
}

/// 检查令牌有效性
#[axum::debug_handler]
pub async fn check_token(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // 检查用户是否存在
    match UserRepository::find_by_id(&state.pool, &claims.sub).await {
        Ok(Some(user)) => {
            // 返回用户信息
            (
                StatusCode::OK,
                success_to_api_response(CheckTokenResponse {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    is_temporary: user.is_temporary,
                }),
            )
        }
        Ok(None) => (
            StatusCode::OK,
            error_to_api_response(error_codes::NOT_FOUND, "用户不存在".to_string()),
        ),
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
        ),
    }
} 