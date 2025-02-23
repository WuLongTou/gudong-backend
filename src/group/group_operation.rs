use crate::common::{ApiResult, MapLocation};
use crate::group::group_types::{
    JoinGroupRequest, JoinGroupResponse, NewGroupRequest, NewGroupResponse,
    QueryGroupInfoRequestByLocation, QueryGroupInfoRequestByName, QueryGroupInfoResponse,
};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

// 路由处理函数

pub(crate) async fn create_group(
    Json(req): Json<NewGroupRequest>,
) -> Json<ApiResult<NewGroupResponse>> {
    let response = NewGroupResponse {
        group_id: Uuid::new_v4().to_string(),
        name: req.name,
        location: req.location,
        location_name: "".to_string(),
        member_count: 1,
    };

    Json(ApiResult {
        code: 0,
        message: None,
        data: response,
    })
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

    Json(ApiResult {
        code: 0,
        message: None,
        data: vec![group],
    })
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

    Json(ApiResult {
        code: 0,
        message: None,
        data: vec![group],
    })
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
    Json(ApiResult {
        code: 0,
        message: None,
        data: response,
    })
}
