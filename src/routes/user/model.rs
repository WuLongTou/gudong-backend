use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

use crate::utils::{generate_recovery_code, hash_password, verify_password};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: String,
    pub nickname: String,
    pub is_temporary: bool,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    #[serde(skip_serializing)]
    pub recovery_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRegisteredUserRequest {
    pub user_id: String,
    pub password: String,
    pub nickname: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemporaryUserRequest {}

#[derive(Debug, Serialize)]
pub struct CreateUserResponse {
    pub user_id: String,
    pub nickname: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub user_id: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user_id: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNicknameRequest {
    pub nickname: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePasswordRequest {
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub user_id: String,
    pub recovery_code: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct CheckTokenResponse {
    pub user_id: String,
    pub is_temporary: bool,
}

impl User {
    pub async fn create(
        pool: &PgPool,
        req: CreateRegisteredUserRequest,
    ) -> Result<Self, sqlx::Error> {
        let password_hash = hash_password(&req.password)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;

        let recovery_code = generate_recovery_code(&req.user_id, &req.password);

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (user_id, nickname, password_hash, recovery_code, is_temporary)
            VALUES ($1, $2, $3, $4, false)
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            req.user_id,
            req.nickname,
            password_hash,
            recovery_code
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn create_temporary(
        pool: &PgPool,
        user_id: &str,
        nickname: &str,
    ) -> Result<Self, sqlx::Error> {
        tracing::debug!("Creating temporary user: {}", user_id);

        let result = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (user_id, nickname, is_temporary)
            VALUES ($1, $2, true)
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            user_id,
            nickname
        )
        .fetch_one(pool)
        .await;

        match result {
            Ok(user) => {
                tracing::info!("Created temporary user: {}", user.user_id);
                Ok(user)
            }
            Err(e) => {
                tracing::error!("Failed to create temporary user: {:?}", e);
                Err(e)
            }
        }
    }

    pub async fn find_by_id(pool: &PgPool, user_id: &str) -> Result<Option<Self>, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT user_id, nickname, password_hash, recovery_code, is_temporary
            FROM users
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    pub async fn verify_login(&self, password: &str) -> Result<bool, bcrypt::BcryptError> {
        match &self.password_hash {
            Some(hash) => verify_password(password, hash),
            None => Ok(false),
        }
    }

    pub async fn reset_password(
        pool: &PgPool,
        req: ResetPasswordRequest,
    ) -> Result<Self, sqlx::Error> {
        let user = Self::find_by_id(pool, &req.user_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        if user.recovery_code.as_deref() != Some(&req.recovery_code) {
            return Err(sqlx::Error::Protocol("Invalid recovery code".into()));
        }

        let password_hash = hash_password(&req.new_password)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;
        let new_recovery_code = generate_recovery_code(&req.user_id, &req.new_password);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET password_hash = $1, recovery_code = $2
            WHERE user_id = $3
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            password_hash,
            new_recovery_code,
            req.user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn update_nickname(
        pool: &PgPool,
        user_id: &str,
        nickname: String,
    ) -> Result<Self, sqlx::Error> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET nickname = $1
            WHERE user_id = $2
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            nickname,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn update_password(
        pool: &PgPool,
        user_id: &str,
        password: String,
    ) -> Result<Self, sqlx::Error> {
        let password_hash = hash_password(&password)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;
        let recovery_code = generate_recovery_code(user_id, &password);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET password_hash = $1, recovery_code = $2
            WHERE user_id = $3
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            password_hash,
            recovery_code,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }
}
