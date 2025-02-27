use axum::{routing::post, Router};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;

mod handlers;
mod types;

pub fn router() -> Router<Pool<RedisConnectionManager>> {
    Router::new()
        .route("/create", post(handlers::create_group))
        .route("/query-by-name", post(handlers::query_groups_by_name))
        .route("/join", post(handlers::join_group))
        .route(
            "/query-by-location",
            post(handlers::query_groups_by_location),
        )
        .route("/send-message", post(handlers::send_message_to_group))
        .route("/query-message", post(handlers::query_message_from_group))
}
