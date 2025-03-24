use axum::{
    Extension,
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::AppState;

use super::model::{CreateGroupRequest, Group, GroupInfo, JoinGroupRequest, KeepAliveRequest};

use crate::utils::{Claims, error_codes, error_to_api_response, success_to_api_response};

#[derive(Debug, Deserialize)]
pub struct LocationQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub radius: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct NameQuery {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub group_id: String,
}

#[derive(Debug, Serialize)]
pub struct KeepAliveResponse {
    pub last_active_time: DateTime<Utc>,
}

#[axum::debug_handler]
pub async fn create_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    match Group::create(&state.pool, req, claims.sub).await {
        Ok(group) => (
            StatusCode::CREATED,
            success_to_api_response(GroupInfo::from(group)),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_id(
    State(state): State<AppState>,
    Query(query): Query<IdQuery>,
) -> impl IntoResponse {
    match Group::find_by_id(&state.pool, &state.redis, &query.group_id).await {
        Ok(Some(group)) => (
            StatusCode::OK,
            success_to_api_response(GroupInfo::from(group)),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            error_to_api_response(error_codes::NOT_FOUND, "Group not found".to_string()),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_name(
    State(state): State<AppState>,
    Query(query): Query<NameQuery>,
) -> impl IntoResponse {
    match Group::find_by_name(&state.pool, &state.redis, &query.name).await {
        Ok(groups) => {
            let group_infos = groups.into_iter().map(GroupInfo::from).collect::<Vec<_>>();
            (StatusCode::OK, success_to_api_response(group_infos))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_location(
    State(state): State<AppState>,
    Query(query): Query<LocationQuery>,
) -> impl IntoResponse {
    let radius = query
        .radius
        .unwrap_or(1000.0)
        .min(state.config.max_search_radius);

    match Group::find_by_location(
        &state.pool,
        &state.redis,
        query.latitude,
        query.longitude,
        radius,
    )
    .await
    {
        Ok(groups) => {
            let group_infos = groups.into_iter().map(GroupInfo::from).collect::<Vec<_>>();
            (StatusCode::OK, success_to_api_response(group_infos))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn join_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<JoinGroupRequest>,
) -> impl IntoResponse {
    match Group::join(
        &state.pool,
        &state.redis,
        &req.group_id,
        &claims.sub,
        req.password,
    )
    .await
    {
        Ok(_) => (
            StatusCode::OK,
            success_to_api_response(serde_json::json!({
                "success": true
            })),
        ),
        Err(e) => {
            let status = if e.to_string().contains("Password required")
                || e.to_string().contains("Invalid password")
            {
                StatusCode::FORBIDDEN
            } else if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn leave_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<IdQuery>,
) -> impl IntoResponse {
    match Group::leave(&state.pool, &state.redis, &req.group_id, &claims.sub).await {
        Ok(_) => (
            StatusCode::OK,
            success_to_api_response(serde_json::json!({
                "success": true
            })),
        ),
        Err(e) => {
            let status = if e.to_string().contains("User not in group") {
                StatusCode::BAD_REQUEST
            } else if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn keep_alive(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<KeepAliveRequest>,
) -> impl IntoResponse {
    match Group::keep_alive(&state.pool, &req.group_id, &claims.sub).await {
        Ok(last_active) => (
            StatusCode::OK,
            success_to_api_response(serde_json::json!({
                "last_active": last_active
            })),
        ),
        Err(e) => {
            let status = if e.to_string().contains("Row not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(error_codes::INTERNAL_ERROR, e.to_string()),
            )
        }
    }
}
