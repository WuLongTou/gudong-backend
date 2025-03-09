use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    AppState,
    models::{
        CreateRegisteredUserRequest, CreateUserResponse, LoginRequest, LoginResponse,
        ResetPasswordRequest, ResetPasswordResponse, UpdateUserRequest, User,
    },
    utils::{Claims, generate_temp_token, generate_token},
};

use super::{error_to_api_response, success_to_api_response};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<CreateRegisteredUserRequest>,
) -> impl IntoResponse {
    // 检查用户ID格式
    if !req.user_id.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return (
            StatusCode::BAD_REQUEST,
            error_to_api_response(
                1,
                "Invalid user ID format. Only alphanumeric characters and underscore are allowed."
                    .to_string(),
            ),
        );
    }

    match User::create(&state.pool, req).await {
        Ok(user) => match generate_token(&user.user_id, &state.config) {
            Ok(token) => (
                StatusCode::CREATED,
                success_to_api_response(CreateUserResponse {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    token,
                }),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_to_api_response(1, "failed to generate token".to_string()),
            ),
        },
        Err(e) => {
            if e.to_string().contains("unique constraint") {
                (
                    StatusCode::CONFLICT,
                    error_to_api_response(1, "User already exists".to_string()),
                )
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error_to_api_response(1, "failed to create user".to_string()),
                )
            }
        }
    }
}

pub async fn create_temporary(State(state): State<AppState>) -> impl IntoResponse {
    tracing::info!("create_temporary");
    match User::create_temporary(&state.pool).await {
        Ok(user) => match generate_temp_token(&user.user_id, &state.config) {
            Ok(token) => (
                StatusCode::CREATED,
                success_to_api_response(CreateUserResponse {
                    user_id: user.user_id,
                    nickname: user.nickname,
                    token,
                }),
            ),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_to_api_response(1, "failed to generate token".to_string()),
            ),
        },

        Err(e) => {
            tracing::error!("Database error: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_to_api_response(1, "failed to create user".to_string()),
            )
        }
    }
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = match User::find_by_id(&state.pool, &req.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                error_to_api_response(1, "User not found".to_string()),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_to_api_response(1, "Database error: {}".to_string()),
            );
        }
    };

    // 检查是否为临时用户
    if user.is_temporary {
        return (
            StatusCode::UNAUTHORIZED,
            error_to_api_response(1, "Temporary user cannot login".to_string()),
        );
    }

    // 验证密码
    match user.verify_login(&req.password).await {
        Ok(true) => (),
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                error_to_api_response(1, "Invalid password".to_string()),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                error_to_api_response(1, "Database error".to_string()),
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
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, "Failed to generate token".to_string()),
        ),
    }
}

pub async fn update_user(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    // 如果要更新密码，检查新密码的长度
    if let Some(ref password) = req.password {
        if password.len() < 6 {
            return (
                StatusCode::BAD_REQUEST,
                error_to_api_response(1, "Password must be at least 6 characters long".to_string()),
            );
        }
    }

    match User::update(&state.pool, &claims.sub, req).await {
        Ok(user) => (StatusCode::OK, success_to_api_response(user)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, e.to_string()),
        ),
    }
}

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
            (status, error_to_api_response(1, e.to_string()))
        }
    }
}
