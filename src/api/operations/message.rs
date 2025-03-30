// 消息处理器
// 处理消息相关的API请求

use crate::AppState;
use crate::api::models::message::*;
use crate::database::operations::message::MessageOperation;
use crate::utils::Claims;
use crate::utils::{error_codes, error_to_api_response, success_to_api_response};
use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use sqlx;
use std::sync::Arc;

/// 发送消息
pub async fn send_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    tracing::debug!(
        "用户(登录ID) {} 正在向群组 {} 发送消息",
        claims.sub,
        payload.group_id
    );

    // 创建消息仓库实例
    let db_operation = MessageOperation::new(Arc::new(state.pool.clone()));

    // 从认证信息中获取用户ID
    let user_id = &claims.sub;

    // 发送消息
    match db_operation
        .save_message(&payload.group_id, user_id, &payload.content)
        .await
    {
        Ok(message_id) => {
            tracing::info!(
                "用户(登录ID) {} 成功向群组 {} 发送消息 {}",
                user_id,
                payload.group_id,
                message_id
            );
            (
                StatusCode::OK,
                success_to_api_response(SendMessageResponse {
                    message_id,
                    sent_at: Utc::now(),
                }),
            )
        }
        Err(e) => {
            if e.to_string().contains("User is not a member") {
                tracing::warn!(
                    "用户(登录ID) {} 向群组 {} 发送消息失败: 不是群组成员",
                    user_id,
                    payload.group_id
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<SendMessageResponse>(
                        error_codes::PERMISSION_DENIED,
                        "用户不是该群组成员".to_string(),
                    ),
                )
            } else {
                tracing::error!(
                    "用户(登录ID) {} 向群组 {} 发送消息失败: {}",
                    user_id,
                    payload.group_id,
                    e
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<SendMessageResponse>(
                        error_codes::INTERNAL_ERROR,
                        format!("发送消息失败: {}", e),
                    ),
                )
            }
        }
    }
}

/// 获取消息历史
pub async fn get_message_history(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(group_id): Path<String>,
    Query(params): Query<GetMessageHistoryPageParams>,
) -> impl IntoResponse {
    let user_id = &claims.sub;
    tracing::debug!("用户(登录ID) {} 正在获取群组 {} 的消息历史", user_id, group_id);

    // 创建消息仓库实例
    let db_operation = MessageOperation::new(Arc::new(state.pool.clone()));

    // 先检查用户是否是群组成员
    let is_member_result = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM group_members 
            WHERE group_id = $1 AND user_id = $2
        ) as "exists!"
        "#,
        group_id,
        user_id
    )
    .fetch_one(&state.pool)
    .await;

    if let Err(err) = is_member_result {
        tracing::error!(
            "检查用户(登录ID) {} 是否为群组 {} 成员时出错: {}",
            user_id,
            group_id,
            err
        );
        return (
            StatusCode::OK,
            error_to_api_response::<GetMessageHistoryResponse>(
                error_codes::INTERNAL_ERROR,
                format!("检查群组成员资格失败: {}", err),
            ),
        );
    }

    let is_member = is_member_result.unwrap().exists;

    if !is_member {
        tracing::warn!(
            "用户(登录ID) {} 尝试获取群组 {} 消息但不是群组成员",
            user_id,
            group_id
        );
        return (
            StatusCode::OK,
            error_to_api_response::<GetMessageHistoryResponse>(
                error_codes::PERMISSION_DENIED,
                "用户不是该群组成员".to_string(),
            ),
        );
    }

    // 获取消息历史
    match db_operation
        .get_group_messages(&group_id, params.limit as i64, params.cursor.as_deref())
        .await
    {
        Ok(messages) => {
            tracing::debug!(
                "用户(登录ID) {} 成功获取群组 {} 的 {} 条消息",
                user_id,
                group_id,
                messages.len()
            );

            // 转换为API响应格式
            let message_details: Vec<MessageDetail> = messages
                .into_iter()
                .map(|msg| {
                    MessageDetail {
                        id: msg.message_id,
                        group_id: msg.group_id,
                        sender_id: msg.user_id,
                        sender_name: msg.nickname,
                        message_type: MessageType::Text, // 暂时只支持文本消息
                        content: msg.content,
                        sent_at: msg.created_at,
                    }
                })
                .collect();

            // 获取下一页游标
            let next_cursor = if !message_details.is_empty() {
                Some(message_details.last().unwrap().id.clone())
            } else {
                None
            };

            // 判断是否还有更多消息
            let has_more = message_details.len() == params.limit as usize;

            (
                StatusCode::OK,
                success_to_api_response(GetMessageHistoryResponse {
                    messages: message_details,
                    next_cursor,
                    has_more,
                }),
            )
        }
        Err(e) => {
            tracing::error!("用户(登录ID) {} 获取群组 {} 消息失败: {}", user_id, group_id, e);
            (
                StatusCode::OK,
                error_to_api_response::<GetMessageHistoryResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("获取消息失败: {}", e),
                ),
            )
        }
    }
}

/// 删除消息
pub async fn delete_message(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    let user_id = &claims.sub;
    tracing::debug!("用户(登录ID) {} 正在尝试删除消息 {}", user_id, message_id);

    // 创建消息仓库实例
    let db_operation = MessageOperation::new(Arc::new(state.pool.clone()));

    // 删除消息
    match db_operation.delete_message(&message_id, user_id).await {
        Ok(deleted) => {
            if deleted {
                tracing::info!("用户(登录ID) {} 成功删除消息 {}", user_id, message_id);
                (
                    StatusCode::OK,
                    success_to_api_response(DeleteMessageResponse { success: true }),
                )
            } else {
                tracing::warn!(
                    "用户(登录ID) {} 无法删除消息 {}: 消息不存在或没有权限",
                    user_id,
                    message_id
                );
                (
                    StatusCode::OK,
                    error_to_api_response::<DeleteMessageResponse>(
                        error_codes::NOT_FOUND,
                        "消息不存在或您没有权限删除".to_string(),
                    ),
                )
            }
        }
        Err(e) => {
            tracing::error!("用户(登录ID) {} 删除消息 {} 失败: {}", user_id, message_id, e);
            (
                StatusCode::OK,
                error_to_api_response::<DeleteMessageResponse>(
                    error_codes::INTERNAL_ERROR,
                    format!("删除消息失败: {}", e),
                ),
            )
        }
    }
}
