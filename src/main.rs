use axum::{extract::Extension, routing::post, Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

// 公共响应结构
#[derive(Serialize)]
struct ApiResult<T> {
    code: i32,
    message: Option<String>,
    data: T,
}

// 群组相关数据结构
#[derive(Deserialize)]
struct NewGroupRequest {
    name: String,
    location: MapLocation,
}

#[derive(Serialize)]
struct NewGroupResponse {
    group_id: String,
    name: String,
    location: MapLocation,
    location_name: String,
    member_count: i32,
}

#[derive(Deserialize)]
struct QueryGroupInfoRequestByName {
    name: String,
}

#[derive(Serialize)]
struct QueryGroupInfoResponseByName {
    group_id: String,
    name: String,
    location: MapLocation,
    location_name: String,
    member_count: i32,
}

#[derive(Deserialize)]
struct JoinGroupRequest {
    group_id: String,
    user_id: String,
    room_access_token: Option<String>,
}

#[derive(Serialize)]
struct JoinGroupResponse {
    success: bool,
    joined_at: Option<String>,
}

// 公共数据结构
#[derive(Deserialize, Serialize, Clone)]
struct MapLocation {
    latitude: f64,
    longitude: f64,
}

// 1. 添加请求结构体（在现有结构体下方添加）
#[derive(Deserialize)]
struct QueryGroupInfoRequestByLocation {
    location: MapLocation,
}

// 路由处理函数
async fn create_group(Json(req): Json<NewGroupRequest>) -> Json<ApiResult<NewGroupResponse>> {
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

async fn query_groups_by_name(
    Json(req): Json<QueryGroupInfoRequestByName>,
) -> Json<ApiResult<Vec<QueryGroupInfoResponseByName>>> {
    let group = QueryGroupInfoResponseByName {
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

// 2. 添加路由处理函数
async fn query_groups_by_location(
    Json(req): Json<QueryGroupInfoRequestByLocation>,
) -> Json<ApiResult<Vec<QueryGroupInfoResponseByName>>> {
    // 模拟返回附近群组数据
    let group = QueryGroupInfoResponseByName {
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

async fn join_group(Json(req): Json<JoinGroupRequest>) -> Json<ApiResult<JoinGroupResponse>> {
    let response = JoinGroupResponse {
        success: true,
        joined_at: Some(Utc::now().to_rfc3339()),
    };

    Json(ApiResult {
        code: 0,
        message: None,
        data: response,
    })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let cors = CorsLayer::permissive();
    let app = Router::new()
        .route("/group/create", post(create_group))
        .route("/group/query-by-name", post(query_groups_by_name))
        .route("/group/query-by-location", post(query_groups_by_location))
        .route("/group/join", post(join_group))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Server is running on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
