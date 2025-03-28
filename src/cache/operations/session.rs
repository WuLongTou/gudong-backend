use std::sync::Arc;
use redis::{Client as RedisClient, AsyncCommands};
use crate::cache::models::session::CachedSession;

/// 会话缓存操作
pub struct SessionCacheOperations;

impl SessionCacheOperations {
    /// 缓存会话
    pub async fn cache_session(
        redis: &Arc<RedisClient>,
        session_id: &str,
        user_id: &str,
        data: Option<String>,
        ttl: u64,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + ttl as i64;
        
        let cached_session = CachedSession {
            session_id: session_id.to_string(),
            user_id: user_id.to_string(),
            data,
            created_at: now,
            expires_at,
        };
        
        let key = format!("session:{}", session_id);
        let json = serde_json::to_string(&cached_session)
            .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "序列化错误", e.to_string())))?;
        
        let _: () = conn.set_ex(key, json, ttl).await?;
        
        Ok(())
    }
    
    /// 获取会话
    pub async fn get_session(
        redis: &Arc<RedisClient>,
        session_id: &str,
    ) -> Result<Option<CachedSession>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let key = format!("session:{}", session_id);
        let result: Option<String> = conn.get(key).await?;
        
        match result {
            Some(json) => {
                let cached_session = serde_json::from_str(&json)
                    .map_err(|e| redis::RedisError::from((redis::ErrorKind::IoError, "反序列化错误", e.to_string())))?;
                Ok(Some(cached_session))
            },
            None => Ok(None),
        }
    }
    
    /// 获取用户的所有会话
    pub async fn get_user_sessions(
        redis: &Arc<RedisClient>,
        user_id: &str,
    ) -> Result<Vec<CachedSession>, redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        // 模式匹配所有会话键
        let keys: Vec<String> = conn.keys("session:*").await?;
        let mut sessions = Vec::new();
        
        for key in keys {
            let result: Option<String> = conn.get(&key).await?;
            if let Some(json) = result {
                if let Ok(session) = serde_json::from_str::<CachedSession>(&json) {
                    if session.user_id == user_id {
                        sessions.push(session);
                    }
                }
            }
        }
        
        Ok(sessions)
    }
    
    /// 删除会话
    pub async fn remove_session(
        redis: &Arc<RedisClient>,
        session_id: &str,
    ) -> Result<(), redis::RedisError> {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        
        let key = format!("session:{}", session_id);
        let _: () = conn.del(key).await?;
        
        Ok(())
    }
    
    /// 刷新会话过期时间
    pub async fn refresh_session(
        redis: &Arc<RedisClient>,
        session_id: &str,
        ttl: u64,
    ) -> Result<(), redis::RedisError> {
        match Self::get_session(redis, session_id).await? {
            Some(session) => {
                Self::cache_session(
                    redis,
                    session_id,
                    &session.user_id,
                    session.data,
                    ttl,
                ).await?;
                Ok(())
            },
            None => Err(redis::RedisError::from((redis::ErrorKind::IoError, "会话不存在", String::new()))),
        }
    }
} 