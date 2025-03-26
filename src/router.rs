use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers;

// 用户相关的路由
pub fn user_routes() -> Router {
    Router::new()
        .route("/users/register", post(handlers::user::register))
        .route("/users/login", post(handlers::user::login))
        .route("/users/me", get(handlers::user::get_me))
        // 更多用户相关路由...
}

// 群组相关的路由
pub fn group_routes() -> Router {
    Router::new()
        .route("/groups", post(handlers::group::create_group))
        .route("/groups", get(handlers::group::list_groups))
        .route("/groups/:group_id", get(handlers::group::get_group))
        // 更多群组相关路由...
}

// 消息相关的路由
pub fn message_routes() -> Router {
    Router::new()
        .route("/messages", post(handlers::message::send_message))
        .route("/messages", get(handlers::message::list_messages))
        // 更多消息相关路由...
}

// 活动相关的路由
pub fn activity_routes() -> Router {
    Router::new()
        .route("/activities", post(handlers::activity::create_activity))
        .route("/activities/nearby", get(handlers::activity::get_nearby_activities))
        .route("/users/nearby", get(handlers::activity::get_nearby_users))
        .route("/users/me/activities", get(handlers::activity::get_my_activities))
        .route("/users/:user_id/activities", get(handlers::activity::get_user_activities))
}

// 创建主路由
pub fn create_router() -> Router {
    Router::new()
        .merge(user_routes())
        .merge(group_routes())
        .merge(message_routes())
        .merge(activity_routes())
        // 可能的API版本前缀和其他中间件...
} 