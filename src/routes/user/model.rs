use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

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
pub struct UpdateUserRequest {
    pub nickname: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub user_id: String,
    pub recovery_code: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {}

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

    pub async fn create_temporary(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let user_id = format!("temp_{}", Uuid::new_v4());
        let nickname = format!("临时用户_{}", &user_id[..8]);

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

    pub async fn update(
        pool: &PgPool,
        user_id: &str,
        req: UpdateUserRequest,
    ) -> Result<Self, sqlx::Error> {
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(nickname) = req.nickname {
            updates.push(format!("nickname = ${}", updates.len() + 1));
            params.push(nickname);
        }

        if let Some(password) = req.password {
            let password_hash = hash_password(&password)
                .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;
            let recovery_code = generate_recovery_code(user_id, &password);

            updates.push(format!("password_hash = ${}", updates.len() + 1));
            params.push(password_hash);
            updates.push(format!("recovery_code = ${}", updates.len() + 1));
            params.push(recovery_code);
        }

        if updates.is_empty() {
            return Self::find_by_id(pool, user_id)
                .await?
                .ok_or_else(|| sqlx::Error::RowNotFound);
        }

        let query = format!(
            r#"
            UPDATE users
            SET {}
            WHERE user_id = ${}
            RETURNING user_id, nickname, password_hash, recovery_code, is_temporary
            "#,
            updates.join(", "),
            params.len() + 1
        );

        let mut query = sqlx::query_as(&query);
        for param in params {
            query = query.bind(param);
        }
        query = query.bind(user_id);

        query.fetch_one(pool).await
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
}
