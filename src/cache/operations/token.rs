use crate::cache::models::token::CachedToken;
use redis::{AsyncCommands, Client as RedisClient};
use std::sync::Arc;

/// 令牌缓存操作
pub struct TokenCacheOperations;

impl TokenCacheOperations {
    /// 缓存令牌
    pub async fn cache_token(
        redis: &Arc<RedisClient>,
        token: &str,
        user_id: &str,
        expires_at: i64,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;

        let cached_token = CachedToken {
            token: token.to_string(),
            user_id: user_id.to_string(),
            expires_at,
        };

        let key = format!("token:{}", token);
        let json = serde_json::to_string(&cached_token).map_err(|e| {
            redis::RedisError::from((redis::ErrorKind::IoError, "序列化错误", e.to_string()))
        })?;

        // 设置缓存，过期时间与token一致
        let ttl = expires_at - chrono::Utc::now().timestamp();
        if ttl > 0 {
            let _: () = conn.set_ex(key, json, ttl as u64).await?;
        }

        Ok(())
    }

    /// 获取令牌缓存
    pub async fn get_cached_token(
        redis: &Arc<RedisClient>,
        token: &str,
    ) -> Result<Option<CachedToken>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;

        let key = format!("token:{}", token);
        let result: Option<String> = conn.get(key).await?;

        match result {
            Some(json) => {
                let cached_token = serde_json::from_str(&json).map_err(|e| {
                    redis::RedisError::from((
                        redis::ErrorKind::IoError,
                        "反序列化错误",
                        e.to_string(),
                    ))
                })?;
                Ok(Some(cached_token))
            }
            None => Ok(None),
        }
    }

    /// 删除令牌缓存
    pub async fn remove_token(
        redis: &Arc<RedisClient>,
        token: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;

        let key = format!("token:{}", token);
        let _: () = conn.del(key).await?;

        Ok(())
    }
}
