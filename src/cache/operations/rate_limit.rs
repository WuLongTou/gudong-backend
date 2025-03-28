use std::sync::Arc;
use redis::{Client as RedisClient, AsyncCommands};
use crate::cache::models::rate_limit::CachedRateLimit;

/// 速率限制缓存操作
pub struct RateLimitCacheOperations;

impl RateLimitCacheOperations {
    /// 获取速率限制计数
    pub async fn get_rate_limit(
        redis: &Arc<RedisClient>,
        key: &str,
    ) -> Result<Option<CachedRateLimit>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let redis_key = format!("rate_limit:{}", key);
        let result: Option<String> = conn.get(redis_key).await?;
        
        match result {
            Some(json) => {
                let cached_rate_limit = serde_json::from_str(&json)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "反序列化错误", e.to_string())))?;
                Ok(Some(cached_rate_limit))
            },
            None => Ok(None),
        }
    }
    
    /// 设置或更新速率限制
    pub async fn set_rate_limit(
        redis: &Arc<RedisClient>,
        key: &str,
        count: u32,
        ttl: u64,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let reset_at = chrono::Utc::now().timestamp() + ttl as i64;
        
        let cached_rate_limit = CachedRateLimit {
            key: key.to_string(),
            count,
            reset_at,
        };
        
        let redis_key = format!("rate_limit:{}", key);
        let json = serde_json::to_string(&cached_rate_limit)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "序列化错误", e.to_string())))?;
        
        let _: () = conn.set_ex(redis_key, json, ttl).await?;
        
        Ok(())
    }
    
    /// 增加速率限制计数
    pub async fn increment_rate_limit(
        redis: &Arc<RedisClient>,
        key: &str,
        ttl: u64,
    ) -> Result<u32, redis::RedisError> {
        match Self::get_rate_limit(redis, key).await? {
            Some(mut rate_limit) => {
                rate_limit.count += 1;
                Self::set_rate_limit(redis, key, rate_limit.count, ttl).await?;
                Ok(rate_limit.count)
            },
            None => {
                Self::set_rate_limit(redis, key, 1, ttl).await?;
                Ok(1)
            },
        }
    }
} 