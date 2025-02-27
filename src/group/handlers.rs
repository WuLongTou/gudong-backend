use crate::error::AppError;
use crate::group::types::{
    JoinGroupRequest, JoinGroupResponse, MapLocation, MessageFromGroupResponse, NewGroupRequest,
    NewGroupResponse, QueryGroupInfoRequestByLocation, QueryGroupInfoRequestByName,
    QueryGroupInfoResponse, QueryMessageFromGroupRequest, SendMessageToGroupRequest,
};
use crate::infrastructure::auth;
use crate::result::ApiResult;
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use chrono::Utc;
use serde_json;
use uuid::Uuid;

// 修改所有处理函数签名，添加State参数
pub(crate) async fn create_group(
    State(pool): State<Pool<RedisConnectionManager>>,
    Json(req): Json<NewGroupRequest>,
) -> Result<Json<ApiResult<NewGroupResponse>>, impl IntoResponse> {
    let group_id = Uuid::new_v4().to_string();
    let response = NewGroupResponse {
        group_id: group_id.clone(),
        name: req.name.clone(),
        location: MapLocation {
            latitude: req.location.latitude,
            longitude: req.location.longitude,
        },
        location_name: "".to_string(),
        member_count: 1,
    };

    // 获取连接
    let mut conn = pool.get().await.map_err(|e| {
        tracing::error!("Redis connection error: {}", e);
        AppError::InternalServerError
    })?;

    // 存储群组信息
    redis::cmd("HSET")
        .arg(format!("group:{}", group_id))
        .arg("name")
        .arg(&req.name)
        .arg("location")
        .arg(serde_json::to_string(&req.location).unwrap())
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Redis operation error: {}", e);
            AppError::FailedToStoreGroup
        })?;

    // 添加群组到地理位置索引
    redis::cmd("GEOADD")
        .arg("groups:geo")
        .arg(req.location.longitude)
        .arg(req.location.latitude)
        .arg(&group_id)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Redis operation error: {}", e);
            AppError::FailedToStoreGroup
        })?;

    // 添加群组ID到集合
    redis::cmd("SADD")
        .arg("groups:ids")
        .arg(&group_id)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Redis operation error: {}", e);
            AppError::FailedToStoreGroup
        })?;

    Ok::<_, AppError>(Json(ApiResult::success(response)))
}

pub(crate) async fn query_groups_by_name(
    Json(req): Json<QueryGroupInfoRequestByName>,
) -> Json<ApiResult<Vec<QueryGroupInfoResponse>>> {
    let group = QueryGroupInfoResponse {
        group_id: "test-id".to_string(),
        name: req.name,
        location: MapLocation {
            latitude: 39.9042,
            longitude: 116.4074,
        },
        location_name: "Beijing".to_string(),
        member_count: 100,
    };
    tracing::info!(
        "+++++ query_groups_by_name: {:?}",
        serde_json::to_string(&group).unwrap()
    );

    Json(ApiResult::success(vec![group]))
}

pub(crate) async fn query_groups_by_location(
    Json(req): Json<QueryGroupInfoRequestByLocation>,
) -> Json<ApiResult<Vec<QueryGroupInfoResponse>>> {
    // 模拟返回附近群组数据
    let group = QueryGroupInfoResponse {
        group_id: Uuid::new_v4().to_string(),
        name: "附近群组".to_string(),
        location: MapLocation {
            latitude: req.location.latitude + 0.001, // 模拟不同位置
            longitude: req.location.longitude + 0.001,
        },
        location_name: "模拟位置".to_string(),
        member_count: 10,
    };

    tracing::info!(
        "Querying groups near location: {:?}",
        serde_json::to_string(&req.location).unwrap()
    );

    Json(ApiResult::success(vec![group]))
}

pub(crate) async fn join_group(
    Json(req): Json<JoinGroupRequest>,
) -> Json<ApiResult<JoinGroupResponse>> {
    let response = JoinGroupResponse {
        success: true,
        joined_at: Some(Utc::now().to_rfc3339()),
    };
    tracing::info!(
        "+++++ join_group: {:?}",
        serde_json::to_string(&req).unwrap()
    );
    Json(ApiResult::success(response))
}

// 发送消息到群组
pub(crate) async fn send_message_to_group(
    State(pool): State<Pool<RedisConnectionManager>>,
    Json(req): Json<SendMessageToGroupRequest>,
) -> Result<Json<ApiResult<()>>, impl IntoResponse> {
    // 从Token获取用户信息
    // let claims = auth::validate_token(&req.token).ok_or_else(|| AppError::Unauthorized)?;

    let message = MessageFromGroupResponse {
        // sender_id: claims.user_id,
        // sender_name: claims.username,
        sender_id: "hello".to_string(),
        sender_name: "world".to_string(),
        message: req.message, // 字段名修正
        timestamp: Utc::now().timestamp(),
    };

    // 获取连接
    let mut conn = pool.get().await.map_err(|e| {
        tracing::error!("Redis connection error: {}", e);
        AppError::InternalServerError
    })?;

    let serialized = serde_json::to_string(&message).map_err(|e| {
        tracing::error!("Serialization error: {}", e);
        AppError::InternalServerError
    })?;

    // 使用Sorted Set存储消息
    redis::cmd("ZADD")
        .arg(format!("messages:{}", req.group_id))
        .arg(message.timestamp)
        .arg(&serialized)
        .query_async::<()>(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Redis operation error: {}", e);
            AppError::FailedToStoreMessage
        })?;
    let response = ApiResult::success(());
    tracing::info!(
        "+++++ send_message_to_group: {:?}",
        serde_json::to_string(&response)
    );
    Ok::<_, AppError>(Json(response))
}

// 查询群组消息
pub(crate) async fn query_message_from_group(
    State(pool): State<Pool<RedisConnectionManager>>,
    Json(req): Json<QueryMessageFromGroupRequest>,
) -> Result<Json<ApiResult<Vec<MessageFromGroupResponse>>>, impl IntoResponse> {
    // 获取连接
    let mut conn = pool.get().await.map_err(|e| {
        tracing::error!("Redis connection error: {}", e);
        AppError::InternalServerError
    })?;

    // 使用ZRANGEBYSCORE按时间范围查询消息
    let raw_messages: Vec<String> = redis::cmd("ZRANGEBYSCORE")
        .arg(format!("messages:{}", req.group_id))
        .arg(req.start_timestamp)
        .arg(req.end_timestamp)
        .query_async(&mut *conn)
        .await
        .map_err(|e| {
            tracing::error!("Redis operation error: {}", e);
            AppError::InternalServerError
        })?;

    let mut messages = Vec::new();
    for raw in raw_messages {
        let msg: MessageFromGroupResponse = serde_json::from_str(&raw).map_err(|e| {
            tracing::error!("Deserialization error: {}", e);
            AppError::FailedToGetMessage
        })?;
        if msg.timestamp >= req.start_timestamp && msg.timestamp <= req.end_timestamp {
            messages.push(msg);
        }
    }

    let response = ApiResult::success(messages);
    tracing::info!(
        "+++++ query_message_from_group: {:?}",
        serde_json::to_string(&response)
    );
    Ok::<_, AppError>(Json(response))
}
