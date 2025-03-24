use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use redis::{AsyncCommands, Client as RedisClient};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageInfo {
    pub message_id: String,
    pub group_id: String,
    pub user_id: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageWithNickName {
    pub message_id: String,
    pub group_id: String,
    pub user_id: String,
    pub nickname: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub group_id: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CreateMessageResponse {}

#[derive(Debug, Deserialize)]
pub struct GetMessagesRequest {
    pub group_id: String,
    pub message_id: Option<String>,
    pub limit: Option<i64>,
}

// 缓存相关的常量
const MESSAGE_CACHE_EXPIRE: u64 = 300; // 消息缓存过期时间，单位秒
const MESSAGE_CACHE_PREFIX: &str = "msg:group:"; // 消息缓存前缀

impl MessageInfo {
    pub async fn create(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        req: CreateMessageRequest,
        user_id: String,
    ) -> Result<Self, sqlx::Error> {
        let message_id = Uuid::new_v4().to_string();

        // 检查用户是否在群组中
        let is_member = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM group_members 
                WHERE group_id = $1 AND user_id = $2
            ) as "exists!"
            "#,
            req.group_id,
            user_id
        )
        .fetch_one(pool)
        .await?
        .exists;

        if !is_member {
            return Err(sqlx::Error::Protocol(
                "User is not a member of this group".into(),
            ));
        }

        let message = sqlx::query_as!(
            MessageInfo,
            r#"
            INSERT INTO messages (message_id, group_id, user_id, content, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING message_id, group_id, user_id, content, created_at
            "#,
            message_id,
            req.group_id,
            user_id,
            req.content
        )
        .fetch_one(pool)
        .await?;

        // 发送新消息后，清除相关的消息缓存
        if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
            let cache_key = format!("{}{}", MESSAGE_CACHE_PREFIX, req.group_id);
            let _: Result<(), redis::RedisError> = conn.del(&cache_key).await;
        }

        Ok(message)
    }

    pub async fn get_messages(
        pool: &PgPool,
        redis: &Arc<RedisClient>,
        req: GetMessagesRequest,
        user_id: &str,
    ) -> Result<Vec<MessageWithNickName>, sqlx::Error> {
        // 检查用户是否在群组中
        let is_member = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM group_members 
                WHERE group_id = $1 AND user_id = $2
            ) as "exists!"
            "#,
            req.group_id,
            user_id
        )
        .fetch_one(pool)
        .await?
        .exists;

        if !is_member {
            return Err(sqlx::Error::Protocol(
                "User is not a member of this group".into(),
            ));
        }

        let limit = req
            .limit
            .and_then(|limit_value| Some(limit_value.min(100).max(-100)))
            .unwrap_or(50);

        // 如果没有指定message_id获取最新消息，检查缓存
        if req.message_id.is_none() && limit.abs() <= 50 {
            let cache_key = format!("{}{}", MESSAGE_CACHE_PREFIX, req.group_id);
            
            // 尝试从缓存获取
            if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
                let cached: redis::RedisResult<String> = conn.get(&cache_key).await;
                
                if let Ok(json_str) = cached {
                    if let Ok(messages) = serde_json::from_str::<Vec<MessageWithNickName>>(&json_str) {
                        tracing::debug!("Get messages from cache: {}", cache_key);
                        return Ok(messages);
                    }
                }
            }
        }

        // 从数据库获取消息
        let messages = if let Some(message_id) = req.message_id {
            if limit >= 0 {
                MessageInfo::get_newer_messages_by_message_id(
                    pool,
                    req.group_id.clone(),
                    message_id,
                    limit.abs(),
                )
                .await?
            } else {
                MessageInfo::get_older_messages_by_message_id(
                    pool,
                    req.group_id.clone(),
                    message_id,
                    limit.abs(),
                )
                .await?
            }
        } else {
            let msgs = MessageInfo::get_messages_from_latest_message(pool, req.group_id.clone(), limit.abs()).await?;
            
            // 最新消息缓存到Redis
            if limit.abs() <= 50 {
                if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
                    let cache_key = format!("{}{}", MESSAGE_CACHE_PREFIX, req.group_id);
                    if let Ok(json_str) = serde_json::to_string(&msgs) {
                        let _: Result<(), redis::RedisError> = conn.set_ex(&cache_key, json_str, MESSAGE_CACHE_EXPIRE).await;
                        tracing::debug!("Set messages to cache: {}", cache_key);
                    }
                }
            }
            
            msgs
        };

        Ok(messages)
    }

    async fn get_older_messages_by_message_id(
        pool: &PgPool,
        group_id: String,
        message_id: String,
        limit: i64,
    ) -> Result<Vec<MessageWithNickName>, sqlx::Error> {
        let messages = sqlx::query_as!(
            MessageWithNickName,
            r#"
                SELECT 
                    m.message_id,
                    m.content,
                    m.created_at,
                    m.group_id,
                    u.nickname,
                    m.user_id
                FROM messages m
                JOIN users u ON m.user_id = u.user_id
                WHERE m.group_id = $1
                    AND m.created_at <= (
                        SELECT created_at 
                        FROM messages 
                        WHERE message_id = $2
                    )
                ORDER BY m.created_at DESC
                LIMIT $3
                "#,
            group_id,
            message_id,
            limit
        )
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    async fn get_newer_messages_by_message_id(
        pool: &PgPool,
        group_id: String,
        message_id: String,
        limit: i64,
    ) -> Result<Vec<MessageWithNickName>, sqlx::Error> {
        let messages = sqlx::query_as!(
            MessageWithNickName,
            r#"
                SELECT 
                    m.message_id,
                    m.content,
                    m.created_at,
                    m.group_id,
                    u.nickname,
                    m.user_id
                FROM messages m
                JOIN users u ON m.user_id = u.user_id
                WHERE m.group_id = $1
                    AND m.created_at >= (
                        SELECT created_at 
                        FROM messages 
                        WHERE message_id = $2
                    )
                ORDER BY m.created_at DESC
                LIMIT $3
                "#,
            group_id,
            message_id,
            limit
        )
        .fetch_all(pool)
        .await?;
        Ok(messages)
    }

    async fn get_messages_from_latest_message(
        pool: &PgPool,
        group_id: String,
        limit: i64,
    ) -> Result<Vec<MessageWithNickName>, sqlx::Error> {
        let messages = sqlx::query_as!(
            MessageWithNickName,
            r#"
            SELECT 
                m.message_id,
                m.content,
                m.created_at,
                m.group_id,
                u.nickname,
                m.user_id
            FROM messages m
            JOIN users u ON m.user_id = u.user_id
            WHERE m.group_id = $1
            ORDER BY m.created_at DESC
            LIMIT $2
            "#,
            group_id,
            limit
        )
        .fetch_all(pool)
        .await?;
        Ok(messages)
    }
}
