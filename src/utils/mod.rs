use axum::Json;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing;
use uuid::Uuid;

use crate::api::models::common::ApiResponse;
use crate::config::Config;

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password.as_bytes(), DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    verify(password.as_bytes(), hash)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // 用户ID
    pub exp: i64,      // 过期时间
    pub iat: i64,      // 签发时间
    pub is_temp: bool, // 临时标识
}

pub fn generate_token(
    user_id: &str,
    config: &Config,
) -> Result<(String, i64), jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(config.jwt_expiration().as_secs() as i64))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        iat: Utc::now().timestamp(),
        is_temp: false,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    Ok((token, expiration))
}

pub fn verify_token(token: &str, config: &Config) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}

pub fn generate_recovery_code(user_id: &str, password: &str) -> String {
    let uuid = Uuid::new_v4();
    let recovery_string = format!("{}:{}:{}", user_id, password, uuid);
    hash_password(&recovery_string).unwrap_or_else(|_| String::new())
}

pub fn generate_temp_token(
    user_id: &str,
    config: &Config,
) -> Result<(String, i64), jsonwebtoken::errors::Error> {
    tracing::debug!("Generating temp token for user: {}", user_id);
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::seconds(
            config.temp_token_expiration_secs as i64,
        ))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        iat: Utc::now().timestamp(),
        is_temp: true,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    tracing::debug!("Generated token: {}", token);
    Ok((token, expiration))
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

pub mod error_codes {
    pub const SUCCESS: i32 = 0;
    pub const VALIDATION_ERROR: i32 = 1000;
    pub const USER_EXISTS: i32 = 1001;
    pub const AUTH_FAILED: i32 = 1002;
    pub const PERMISSION_DENIED: i32 = 1003;
    pub const NOT_FOUND: i32 = 1004;
    pub const RATE_LIMIT: i32 = 1005;
    pub const INTERNAL_ERROR: i32 = 5000;
}

/// 根据真实用户ID生成公开ID
///
/// 使用单向哈希+截断的方法，生成一个不可逆但稳定的公开ID
/// 这样可以保护用户的真实ID，同时确保同一用户的公开ID在不同请求中保持一致
pub fn generate_public_id(real_id: &str, salt: &str) -> String {
    // 将用户ID与盐值拼接后计算哈希
    let mut hasher = Sha256::new();
    hasher.update(real_id.as_bytes());
    hasher.update(salt.as_bytes());

    // 获取哈希结果并转换为十六进制字符串，取前12位
    let result = format!("{:x}", hasher.finalize());
    result[..12].to_string()
}

/// 默认用于生成公开用户ID的盐值
pub const PUBLIC_USER_ID_SALT: &str = "3a91b3cd4e7f8b2d5c6a8f1e0d9c7b4a";
