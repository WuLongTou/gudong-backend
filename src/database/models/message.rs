// 消息实体
// 定义消息相关的数据库实体

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 消息类型枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i32)]
pub enum MessageType {
    /// 文本消息
    Text = 0,
    /// 图片消息
    Image = 1,
    /// 语音消息
    Voice = 2,
    /// 系统消息
    System = 10,
}

/// 消息实体，对应数据库中的消息表
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MessageEntity {
    /// 消息ID
    pub id: String,
    /// 群组ID
    pub group_id: String,
    /// 发送者ID
    pub sender_id: String,
    /// 消息类型
    #[sqlx(rename = "message_type")]
    pub message_type: i32,
    /// 消息内容
    pub content: String,
    /// 图片或语音URL
    pub media_url: Option<String>,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
    /// 是否已读
    pub is_read: bool,
    /// 消息顺序号（用于排序）
    pub sequence: i64,
}

impl MessageEntity {
    /// 获取消息类型
    pub fn get_message_type(&self) -> MessageType {
        match self.message_type {
            0 => MessageType::Text,
            1 => MessageType::Image,
            2 => MessageType::Voice,
            10 => MessageType::System,
            _ => MessageType::Text, // 默认为文本消息
        }
    }

    /// 设置消息类型
    pub fn set_message_type(&mut self, message_type: MessageType) {
        self.message_type = message_type as i32;
    }
}

/// 带用户昵称的消息实体
pub struct MessageWithUser {
    pub message_id: String,
    pub group_id: String,
    /// 用户的公开ID，而非登录ID
    pub user_id: String,
    pub nickname: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}
