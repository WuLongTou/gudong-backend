use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    AppState,
    utils::{
        Claims, error_codes, error_to_api_response, generate_temp_token, generate_token,
        success_to_api_response,
    },
};

use super::model::{
    CreateRegisteredUserRequest, CreateUserResponse, LoginRequest, LoginResponse,
    ResetPasswordRequest, ResetPasswordResponse, UpdateNicknameRequest, UpdatePasswordRequest,
    User, RefreshTokenResponse, CheckTokenResponse,
};

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

    match User::create(&state.pool, req).await {
        Ok(user) => {
            // 生成 token
            match generate_token(&user.user_id, &state.config) {
                Ok(token) => (
                    StatusCode::OK,
                    success_to_api_response(CreateUserResponse {
                        user_id: user.user_id,
                        nickname: user.nickname,
                        token,
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

#[axum::debug_handler]
pub async fn create_temporary(State(state): State<AppState>) -> impl IntoResponse {
    // 生成随机用户ID和昵称
    let user_id = uuid::Uuid::new_v4().to_string();
    let nickname = format!("用户{}", &user_id[0..6]);

    match User::create_temporary(&state.pool, &user_id, &nickname).await {
        Ok(user) => {
            // 生成临时token
            match generate_temp_token(&user.user_id, &state.config) {
                Ok(token) => (
                    StatusCode::OK,
                    success_to_api_response(CreateUserResponse {
                        user_id: user.user_id,
                        nickname: user.nickname,
                        token,
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

#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = match User::find_by_id(&state.pool, &req.user_id).await {
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

    // 检查是否为临时用户
    if user.is_temporary {
        return (
            StatusCode::OK,
            error_to_api_response(
                error_codes::AUTH_FAILED,
                "临时用户不能使用密码登录".to_string(),
            ),
        );
    }

    // 验证密码
    match user.verify_login(&req.password).await {
        Ok(true) => (),
        Ok(false) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::AUTH_FAILED, "密码无效".to_string()),
            );
        }
        Err(_) => {
            return (
                StatusCode::OK,
                error_to_api_response(error_codes::INTERNAL_ERROR, "数据库错误".to_string()),
            );
        }
    }

    // 生成 token
    match generate_token(&user.user_id, &state.config) {
        Ok(token) => (
            StatusCode::OK,
            success_to_api_response(LoginResponse {
                user_id: user.user_id,
                token,
            }),
        ),
        Err(_) => (
            StatusCode::OK,
            error_to_api_response(error_codes::INTERNAL_ERROR, "生成令牌失败".to_string()),
        ),
    }
}

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
                "昵称长度必须在2到24个字符之间".to_string(),
            ),
        );
    }

    match User::update_nickname(&state.pool, &claims.sub, req.nickname).await {
        Ok(user) => (StatusCode::OK, success_to_api_response(user)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn update_password(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdatePasswordRequest>,
) -> impl IntoResponse {
    // 验证密码长度
    if req.password.len() < 6 || req.password.len() > 24 {
        return (
            StatusCode::BAD_REQUEST,
            error_to_api_response(
                error_codes::VALIDATION_ERROR,
                "密码长度必须在6到24个字符之间".to_string(),
            ),
        );
    }

    // 对临时用户的检查已经在中间件中完成

    match User::update_password(&state.pool, &claims.sub, req.password).await {
        Ok(user) => (StatusCode::OK, success_to_api_response(user)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<ResetPasswordRequest>,
) -> impl IntoResponse {
    match User::reset_password(&state.pool, req).await {
        Ok(_) => (
            StatusCode::OK,
            success_to_api_response(ResetPasswordResponse {}),
        ),
        Err(e) => {
            let status = if e.to_string().contains("Invalid recovery code") {
                StatusCode::UNAUTHORIZED
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
pub async fn refresh_token(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 根据用户类型使用对应的token生成函数
    let token_result = if claims.is_temp {
        generate_temp_token(&claims.sub, &state.config)
    } else {
        generate_token(&claims.sub, &state.config)
    };

    match token_result {
        Ok(token) => (
            StatusCode::OK,
            success_to_api_response(RefreshTokenResponse {
                token,
            }),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, "刷新令牌失败".to_string()),
        ),
    }
}

/// 检查token是否有效，如果有效返回成功，否则返回失败
#[axum::debug_handler]
pub async fn check_token(
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    // Claims中间件已验证token有效，所以直接返回成功
    (
        StatusCode::OK,
        success_to_api_response(CheckTokenResponse {
            user_id: claims.sub,
            is_temporary: claims.is_temp,
        })
    )
}
