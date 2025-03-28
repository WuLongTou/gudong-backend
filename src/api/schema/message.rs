// 消息相关的数据结构定义

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 消息类型枚举
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 文本消息
    Text,
    /// 图片消息
    Image,
    /// 语音消息
    Voice,
    /// 系统消息
    System,
}

/// 发送消息请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// 群组ID
    pub group_id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
}

/// 发送消息响应
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    /// 消息ID
    pub message_id: String,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
}

/// 获取消息历史请求
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessageHistoryRequest {
    /// 群组ID
    pub group_id: String,
    /// 分页标记（上一页最后一条消息的ID）
    pub cursor: Option<String>,
    /// 消息数量限制
    pub limit: u32,
}

/// 消息详情
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageDetail {
    /// 消息ID
    pub id: String,
    /// 群组ID
    pub group_id: String,
    /// 发送者ID
    pub sender_id: String,
    /// 发送者名称
    pub sender_name: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
}

/// 获取消息历史响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessageHistoryResponse {
    /// 消息列表
    pub messages: Vec<MessageDetail>,
    /// 下一页游标
    pub next_cursor: Option<String>,
    /// 是否还有更多消息
    pub has_more: bool,
} 