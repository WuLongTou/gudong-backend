use crate::database::models::user::UserEntity;
use crate::utils::{
    PUBLIC_USER_ID_SALT, generate_public_id, generate_recovery_code, hash_password,
};
use sqlx::PgPool;

/// 用户存储库实现
pub struct UserOperation;

impl UserOperation {
    /// 创建注册用户
    pub async fn create_registered_user(
        pool: &PgPool,
        user_id: &str,
        nickname: &str,
        password: &str,
    ) -> Result<UserEntity, sqlx::Error> {
        let password_hash = hash_password(password)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;

        let recovery_code = generate_recovery_code(user_id, password);

        // 生成用户公开ID
        let public_user_id = generate_public_id(user_id, PUBLIC_USER_ID_SALT);

        let user = sqlx::query_as!(
            UserEntity,
            r#"
            INSERT INTO users (user_id, nickname, password_hash, recovery_code, is_temporary, public_user_id)
            VALUES ($1, $2, $3, $4, false, $5)
            RETURNING 
                user_id as "user_id!", 
                nickname as "nickname!", 
                is_temporary, 
                password_hash, 
                recovery_code, 
                created_at,
                public_user_id as "public_user_id!"
            "#,
            user_id,
            nickname,
            password_hash,
            recovery_code,
            public_user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// 创建临时用户
    pub async fn create_temporary_user(
        pool: &PgPool,
        user_id: &str,
        nickname: &str,
    ) -> Result<UserEntity, sqlx::Error> {
        tracing::debug!("Creating temporary user: {}", user_id);

        // 生成用户公开ID
        let public_user_id = generate_public_id(user_id, PUBLIC_USER_ID_SALT);

        let result = sqlx::query_as!(
            UserEntity,
            r#"
            INSERT INTO users (user_id, nickname, is_temporary, public_user_id)
            VALUES ($1, $2, true, $3)
            RETURNING 
                user_id as "user_id!", 
                nickname as "nickname!", 
                is_temporary, 
                password_hash, 
                recovery_code, 
                created_at,
                public_user_id as "public_user_id!"
            "#,
            user_id,
            nickname,
            public_user_id
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

    /// 根据ID查找用户
    pub async fn find_by_id(
        pool: &PgPool,
        user_id: &str,
    ) -> Result<Option<UserEntity>, sqlx::Error> {
        let user = sqlx::query_as!(
            UserEntity,
            r#"
            SELECT 
                user_id as "user_id!", 
                nickname as "nickname!", 
                is_temporary, 
                password_hash, 
                recovery_code, 
                created_at,
                public_user_id as "public_user_id!"
            FROM users
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    /// 更新用户昵称
    pub async fn update_nickname(
        pool: &PgPool,
        user_id: &str,
        nickname: &str,
    ) -> Result<UserEntity, sqlx::Error> {
        let user = sqlx::query_as!(
            UserEntity,
            r#"
            UPDATE users
            SET nickname = $1
            WHERE user_id = $2
            RETURNING 
                user_id as "user_id!", 
                nickname as "nickname!", 
                is_temporary, 
                password_hash, 
                recovery_code, 
                created_at,
                public_user_id as "public_user_id!"
            "#,
            nickname,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// 更新用户密码
    pub async fn update_password(
        pool: &PgPool,
        user_id: &str,
        password: &str,
    ) -> Result<UserEntity, sqlx::Error> {
        let password_hash = hash_password(password)
            .map_err(|e| sqlx::Error::Protocol(format!("Failed to hash password: {}", e)))?;
        let recovery_code = generate_recovery_code(user_id, password);

        let user = sqlx::query_as!(
            UserEntity,
            r#"
            UPDATE users
            SET password_hash = $1, recovery_code = $2
            WHERE user_id = $3
            RETURNING 
                user_id as "user_id!", 
                nickname as "nickname!", 
                is_temporary, 
                password_hash, 
                recovery_code, 
                created_at,
                public_user_id as "public_user_id!"
            "#,
            password_hash,
            recovery_code,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// 重置用户密码
    pub async fn reset_password(
        pool: &PgPool,
        user_id: &str,
        recovery_code: &str,
        new_password: &str,
    ) -> Result<UserEntity, sqlx::Error> {
        // 先查找用户
        let user = Self::find_by_id(pool, user_id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        // 验证恢复码
        if user.recovery_code.as_deref() != Some(recovery_code) {
            return Err(sqlx::Error::Protocol("Invalid recovery code".into()));
        }

        // 更新密码
        Self::update_password(pool, user_id, new_password).await
    }
}
