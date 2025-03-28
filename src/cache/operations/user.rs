use std::sync::Arc;
use redis::{Client as RedisClient, AsyncCommands};
use crate::cache::models::user::{CachedUser, CachedUserStatus, CachedUserLocation};
use crate::database::entities::user::UserEntity;
use crate::cache::keys::user_keys;

/// 用户缓存操作
pub struct UserCacheOperations;

impl UserCacheOperations {
    /// 将用户信息缓存到 Redis
    pub async fn cache_user(redis: &Arc<RedisClient>, user: &UserEntity) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let cached_user = CachedUser {
            user_id: user.user_id.clone(),
            nickname: user.nickname.clone(),
            is_temporary: user.is_temporary,
            created_at: user.created_at.timestamp(),
        };
        
        let key = user_keys::user_info_key(&user.user_id);
        let json = serde_json::to_string(&cached_user)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Serialization error", e.to_string())))?;
        
        let _: () = conn.set_ex(key, json, 3600).await?; // 缓存1小时
        
        Ok(())
    }
    
    /// 从 Redis 获取用户信息
    pub async fn get_cached_user(redis: &Arc<RedisClient>, user_id: &str) -> Result<Option<CachedUser>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let key = user_keys::user_info_key(user_id);
        let result: Option<String> = conn.get(key).await?;
        
        match result {
            Some(json) => {
                let cached_user = serde_json::from_str(&json)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Deserialization error", e.to_string())))?;
                Ok(Some(cached_user))
            }
            None => Ok(None),
        }
    }
    
    /// 更新用户在线状态
    pub async fn update_user_status(
        redis: &Arc<RedisClient>, 
        user_id: &str, 
        online: bool,
        location: Option<(f64, f64)>
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let now = chrono::Utc::now().timestamp();
        let location = location.map(|(latitude, longitude)| CachedUserLocation {
            latitude,
            longitude,
            update_time: now,
        });
        
        let status = CachedUserStatus {
            user_id: user_id.to_string(),
            online,
            last_activity: now,
            location,
        };
        
        let key = user_keys::user_status_key(user_id);
        let json = serde_json::to_string(&status)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Serialization error", e.to_string())))?;
        
        let _: () = conn.set_ex(key, json, 3600).await?; // 缓存1小时
        
        Ok(())
    }
    
    /// 获取用户在线状态
    pub async fn get_user_status(redis: &Arc<RedisClient>, user_id: &str) -> Result<Option<CachedUserStatus>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let key = user_keys::user_status_key(user_id);
        let result: Option<String> = conn.get(key).await?;
        
        match result {
            Some(json) => {
                let status = serde_json::from_str(&json)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "Deserialization error", e.to_string())))?;
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }
    
    /// 从缓存中删除用户
    pub async fn remove_user_from_cache(redis: &Arc<RedisClient>, user_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let info_key = user_keys::user_info_key(user_id);
        let status_key = user_keys::user_status_key(user_id);
        
        let _: () = conn.del(&[info_key, status_key]).await?;
        
        Ok(())
    }
} 