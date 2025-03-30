// 消息相关的数据结构定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 消息类型
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

/// 发送群组消息请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SendGroupMessageRequest {
    /// 群组ID
    pub group_id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息内容
    pub content: String,
}

/// 消息发送响应
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageSendResponse {
    /// 消息ID
    pub message_id: String,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
}

/// 查询群组消息历史请求
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryGroupMessageHistoryRequest {
    /// 群组ID
    pub group_id: String,
    /// 分页标记（上一页最后一条消息的ID）
    pub cursor: Option<String>,
    /// 消息数量限制
    pub limit: u32,
}

/// 删除消息响应
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteMessageResponse {
    pub success: bool,
}

/// 获取消息历史分页参数（用于路径参数版本的API）
#[derive(Debug, Serialize, Deserialize)]
pub struct GetMessageHistoryPageParams {
    /// 分页标记（上一页最后一条消息的ID）
    pub cursor: Option<String>,
    /// 消息数量限制
    pub limit: u32,
}

/// 消息详细信息
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageDetailedInfo {
    /// 消息ID
    pub id: String,
    /// 群组ID
    pub group_id: String,
    /// 发送者ID (公开ID，非登录ID)
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

/// 群组消息历史响应
#[derive(Debug, Serialize, Deserialize)]
pub struct GroupMessageHistoryResponse {
    /// 消息列表
    pub messages: Vec<MessageDetailedInfo>,
    /// 下一页游标
    pub next_cursor: Option<String>,
    /// 是否还有更多消息
    pub has_more: bool,
}

/// 消息详情 (别名，与MessageDetailedInfo相同)
pub type MessageDetail = MessageDetailedInfo;

/// 获取消息历史响应 (别名，与GroupMessageHistoryResponse相同)
pub type GetMessageHistoryResponse = GroupMessageHistoryResponse;

/// 发送消息请求 (别名，与SendGroupMessageRequest相同)
pub type SendMessageRequest = SendGroupMessageRequest;

/// 发送消息响应 (别名，与MessageSendResponse相同)
pub type SendMessageResponse = MessageSendResponse;
