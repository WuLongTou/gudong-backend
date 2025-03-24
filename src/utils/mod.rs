use axum::Json;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use tracing;
use uuid::Uuid;

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
) -> Result<String, jsonwebtoken::errors::Error> {
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

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
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

// 计算两个地理坐标点之间的距离（米）
pub fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS: f64 = 6371000.0; // 地球半径（米）

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin() * (delta_lat / 2.0).sin()
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin() * (delta_lon / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS * c
}

pub fn generate_temp_token(
    user_id: &str,
    config: &Config,
) -> Result<String, jsonwebtoken::errors::Error> {
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
    );

    match &token {
        Ok(t) => tracing::debug!("Generated token: {}", t),
        Err(e) => tracing::error!("Token generation failed: {:?}", e),
    }

    token
}

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