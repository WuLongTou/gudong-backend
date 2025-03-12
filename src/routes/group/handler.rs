use axum::{
    Extension,
    extract::{Json, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{AppState, utils::Claims};

use super::model::{CreateGroupRequest, Group, GroupInfo, JoinGroupRequest, KeepAliveRequest};

use crate::utils::{error_to_api_response, success_to_api_response};

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
        Ok(group) => {
            let response: GroupInfo = group.into();
            (StatusCode::CREATED, success_to_api_response(response))
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, format!("internal server error")),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_id(
    State(state): State<AppState>,
    Query(query): Query<IdQuery>,
) -> impl IntoResponse {
    match Group::find_by_id(&state.pool, &query.group_id).await {
        Ok(Some(group)) => {
            let response: GroupInfo = group.into();
            (StatusCode::OK, success_to_api_response(response))
        }
        Ok(None) => (
            StatusCode::OK,
            error_to_api_response(100, "group not found".to_string()),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(500, "internal server error".to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn find_by_name(
    State(state): State<AppState>,
    Query(query): Query<NameQuery>,
) -> impl IntoResponse {
    match Group::find_by_name(&state.pool, &query.name).await {
        Ok(groups) => {
            let responses: Vec<GroupInfo> = groups.into_iter().map(Into::into).collect();
            (StatusCode::OK, success_to_api_response(responses))
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, "internal server error".to_string()),
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

    match Group::find_by_location(&state.pool, query.latitude, query.longitude, radius).await {
        Ok(groups) => {
            let responses: Vec<GroupInfo> = groups.into_iter().map(Into::into).collect();
            (StatusCode::OK, success_to_api_response(responses))
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, "internal server error".to_string()),
        ),
    }
}

#[axum::debug_handler]
pub async fn join_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<JoinGroupRequest>,
) -> impl IntoResponse {
    match Group::join(&state.pool, &req.group_id, &claims.sub, req.password).await {
        Ok(()) => (StatusCode::OK, success_to_api_response(None::<()>)),
        Err(e) => {
            let status = if e.to_string().contains("Password required")
                || e.to_string().contains("Invalid password")
            {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(1, "internal server error".to_string()),
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
    match Group::leave(&state.pool, &req.group_id, &claims.sub).await {
        Ok(()) => (StatusCode::OK, success_to_api_response(None::<()>)),
        Err(e) => {
            let status = if e.to_string().contains("User not in group") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                status,
                error_to_api_response(1, "internal server error".to_string()),
            )
        }
    }
}

#[axum::debug_handler]
pub async fn keep_alive(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<KeepAliveRequest>,
) -> impl IntoResponse {
    match Group::keep_alive(&state.pool, &req.group_id, &claims.sub).await {
        Ok(_) => (StatusCode::OK, success_to_api_response(None::<()>)),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            error_to_api_response(1, "internal server error".to_string()),
        ),
    }
}
