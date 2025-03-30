// 消息存储库
// 包含消息相关的数据库操作

use crate::database::models::message::MessageWithUser;
use sqlx::{Error as SqlxError, PgPool};
use std::sync::Arc;
use uuid::Uuid;

/// 消息存储库，处理所有与消息相关的数据库操作
pub struct MessageOperation {
    db: Arc<PgPool>,
}

impl MessageOperation {
    /// 创建新的消息存储库实例
    pub fn new(db: Arc<PgPool>) -> Self {
        Self { db }
    }

    /// 保存消息
    pub async fn save_message(
        &self,
        group_id: &str,
        user_id: &str,
        content: &str,
    ) -> Result<String, SqlxError> {
        // 先检查用户是否在群组中
        let is_member = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM group_members 
                WHERE group_id = $1 AND user_id = $2
            ) as "exists!"
            "#,
            group_id,
            user_id
        )
        .fetch_one(&*self.db)
        .await?
        .exists;

        if !is_member {
            return Err(SqlxError::Protocol(
                "User is not a member of this group".into(),
            ));
        }

        // 获取用户的公开ID
        let user_public_id = sqlx::query!(
            r#"
            SELECT public_user_id FROM users
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&*self.db)
        .await?
        .public_user_id;

        let message_id = Uuid::new_v4().to_string();

        // 使用公开ID作为user_id存储
        sqlx::query!(
            r#"
            INSERT INTO messages (message_id, group_id, user_id, content, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
            message_id,
            group_id,
            user_public_id, // 使用公开ID
            content
        )
        .execute(&*self.db)
        .await?;

        Ok(message_id)
    }

    /// 获取群组消息历史
    pub async fn get_group_messages(
        &self,
        group_id: &str,
        limit: i64,
        before_id: Option<&str>,
    ) -> Result<Vec<MessageWithUser>, SqlxError> {
        let actual_limit = if limit <= 0 { 50 } else { limit.min(100) };

        if let Some(message_id) = before_id {
            // 获取指定消息ID之前的消息
            sqlx::query_as!(
                MessageWithUser,
                r#"
                SELECT 
                    m.message_id,
                    m.group_id,
                    m.user_id,
                    u.nickname,
                    m.content,
                    m.created_at
                FROM messages m
                JOIN users u ON m.user_id = u.public_user_id
                WHERE m.group_id = $1
                AND m.created_at < (
                    SELECT created_at FROM messages 
                    WHERE message_id = $2 AND group_id = $1
                )
                ORDER BY m.created_at DESC
                LIMIT $3
                "#,
                group_id,
                message_id,
                actual_limit
            )
            .fetch_all(&*self.db)
            .await
        } else {
            // 获取最新消息
            sqlx::query_as!(
                MessageWithUser,
                r#"
                SELECT 
                    m.message_id,
                    m.group_id,
                    m.user_id,
                    u.nickname,
                    m.content,
                    m.created_at
                FROM messages m
                JOIN users u ON m.user_id = u.public_user_id
                WHERE m.group_id = $1
                ORDER BY m.created_at DESC
                LIMIT $2
                "#,
                group_id,
                actual_limit
            )
            .fetch_all(&*self.db)
            .await
        }
    }

    /// 获取单条消息
    pub async fn get_message(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageWithUser>, SqlxError> {
        let message = sqlx::query_as!(
            MessageWithUser,
            r#"
            SELECT 
                m.message_id,
                m.group_id,
                m.user_id,
                u.nickname,
                m.content,
                m.created_at
            FROM messages m
            JOIN users u ON m.user_id = u.public_user_id
            WHERE m.message_id = $1
            "#,
            message_id
        )
        .fetch_optional(&*self.db)
        .await?;

        Ok(message)
    }

    /// 获取指定消息ID之后的消息
    pub async fn get_newer_messages(
        &self,
        group_id: &str,
        message_id: &str,
        limit: i64,
    ) -> Result<Vec<MessageWithUser>, SqlxError> {
        let actual_limit = if limit <= 0 { 50 } else { limit.min(100) };

        let messages = sqlx::query_as!(
            MessageWithUser,
            r#"
            SELECT 
                m.message_id,
                m.group_id,
                m.user_id,
                u.nickname,
                m.content,
                m.created_at
            FROM messages m
            JOIN users u ON m.user_id = u.public_user_id
            WHERE m.group_id = $1
            AND m.created_at > (
                SELECT created_at FROM messages 
                WHERE message_id = $2 AND group_id = $1
            )
            ORDER BY m.created_at ASC
            LIMIT $3
            "#,
            group_id,
            message_id,
            actual_limit
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(messages)
    }

    /// 删除消息
    pub async fn delete_message(&self, message_id: &str, user_id: &str) -> Result<bool, SqlxError> {
        // 获取用户的公开ID
        let user_public_id = sqlx::query!(
            r#"
            SELECT public_user_id FROM users
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&*self.db)
        .await?
        .public_user_id;
        
        // 检查消息是否属于该用户
        let message = sqlx::query!(
            r#"
            SELECT user_id FROM messages 
            WHERE message_id = $1
            "#,
            message_id
        )
        .fetch_optional(&*self.db)
        .await?;

        let Some(msg) = message else {
            return Ok(false); // 消息不存在
        };

        // 只有消息发送者才能删除 - 比较公开ID
        if msg.user_id != user_public_id {
            return Err(SqlxError::Protocol(
                "User cannot delete this message".into(),
            ));
        }

        let result = sqlx::query!(
            r#"
            DELETE FROM messages
            WHERE message_id = $1
            "#,
            message_id
        )
        .execute(&*self.db)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 计算群组中的消息数量
    pub async fn count_group_messages(&self, group_id: &str) -> Result<i64, SqlxError> {
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count FROM messages
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_one(&*self.db)
        .await?;

        Ok(count.count.unwrap_or(0))
    }
}
